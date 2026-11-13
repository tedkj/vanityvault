//! The optional online backup.
//!
//! Everything is locked on this PC before it is sent, so the server stores boxes it cannot open:
//! - password -> Argon2id (salt kept by the server) -> master key -> HKDF-SHA256 -> two keys:
//!   `auth`, which is sent to prove who is signing in and decrypts nothing, and `kek`, which
//!   never leaves this PC;
//! - a random 32-byte vault key per account, stored on the server wrapped with `kek`, and a
//!   second time wrapped with a key made from the printed recovery key;
//! - each key is XChaCha20-Poly1305 under the vault key. The AAD names the account and the entry,
//!   so the server can't move a box to another entry or another account unnoticed;
//! - an entry's id is derived from the vault key and the address, so backing a key up twice
//!   overwrites the same box, and the id tells the server nothing.
//!
//! The server is not trusted: every box it returns is opened, checked against its id, and the
//! secret is checked against its address before it is shown or restored. Password settings from
//! the server are refused when they are weaker than `MIN_KDF`.
//!
//! This is the only part of the app that uses the network, and only after the user says OK: every
//! request is shown in full in the app first (`Asking`), and nothing is sent until it is approved.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};
use zeroize::Zeroizing;

use crate::engine::SharedVault;
use crate::vault::{self, derive, hex, random, seal, unseal, Entry, EntryView, Kdf, Sealed, Secret};

/// Where the backup lives.
const API: &str = "https://api.vanityvault.no";
const USER_AGENT: &str = concat!("VanityVault/", env!("CARGO_PKG_VERSION"));
const PER_CALL: usize = 200;
const MAX_PAGES: usize = 40;
const BAD_ANSWER: &str = "The VanityCloud server sent an answer this app could not read.";

/// The weakest password settings the app accepts from a server.
const MIN_KDF: Kdf = Kdf { m_kib: 65_536, t: 2, p: 1 };

pub struct WireError {
    pub status: u16,
    pub message: String,
}

/// How a request reaches the server, so the transport can be swapped out.
pub trait Wire: Send {
    fn call(&self, path: &str, token: Option<&str>, body: Value) -> Result<Value, WireError>;
}

pub struct Http {
    base: String,
    agent: ureq::Agent,
}

impl Http {
    /// `VANITYVAULT_API` can point the app at another server, for development: an https address,
    /// or plain http on this PC only.
    pub fn from_env() -> Self {
        let local = |u: &str| u.starts_with("http://127.0.0.1:") || u.starts_with("http://localhost:");
        let base = std::env::var("VANITYVAULT_API").ok().filter(|u| u.starts_with("https://") || local(u)).unwrap_or_else(|| API.into());
        Self::new(&base)
    }

    pub fn new(base: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .redirects(0)
            .user_agent(USER_AGENT)
            .build();
        Self { base: base.trim_end_matches('/').to_string(), agent }
    }

    /// Sends `raw`, a JSON text, exactly as it is.
    fn send(&self, path: &str, token: Option<&str>, raw: &str) -> Result<Value, WireError> {
        let mut req = self.agent.post(&format!("{}/v1/{path}", self.base)).set("Content-Type", "application/json");
        if let Some(t) = token {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        match req.send_string(raw) {
            Ok(resp) => resp.into_json().map_err(|_| WireError { status: 0, message: BAD_ANSWER.into() }),
            Err(ureq::Error::Status(status, resp)) => {
                let said = resp.into_json::<Value>().ok().and_then(|v| v["error"].as_str().map(|s| s.chars().take(200).collect::<String>()));
                Err(WireError { status, message: said.unwrap_or_else(|| format!("The VanityCloud server answered with error {status}.")) })
            }
            Err(_) => Err(WireError { status: 0, message: "Could not reach the VanityCloud server. Check your internet connection.".into() }),
        }
    }
}

impl Wire for Http {
    fn call(&self, path: &str, token: Option<&str>, body: Value) -> Result<Value, WireError> {
        self.send(path, token, &body.to_string())
    }
}

const CANCELLED: &str = "Cancelled. Nothing was sent.";

/// The answers to pending `net-ask` questions, and the "don't ask again until the app closes" tick.
#[derive(Default)]
pub struct Ask {
    pending: Mutex<HashMap<u64, mpsc::Sender<bool>>>,
    next: AtomicU64,
    trusted: AtomicBool,
}
pub type SharedAsk = Arc<Ask>;

/// A wire that asks first, for full transparency. Each request is turned into its JSON text once;
/// that text goes to the window as a `net-ask` event and waits for `net_answer`. Only OK sends it,
/// and what is sent is that same text, byte for byte. No answer within ten minutes counts as Cancel.
pub struct Asking {
    inner: Http,
    ask: SharedAsk,
    app: AppHandle,
}

impl Asking {
    pub fn new(inner: Http, ask: SharedAsk, app: AppHandle) -> Self {
        Self { inner, ask, app }
    }

    fn approved(&self, path: &str, token: Option<&str>, raw: &str) -> bool {
        // Fetching the locked boxes (`{"after":""}`) and signing out (`{}`) carry no data, only the
        // session pass, so they don't ask. The app's texts say so.
        if path == "entries/list" || path == "logout" || self.ask.trusted.load(Ordering::SeqCst) {
            return true;
        }
        let id = self.ask.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.ask.pending.lock().unwrap().insert(id, tx);
        let shown = json!({
            "id": id,
            "path": path,
            "url": format!("{}/v1/{path}", self.inner.base),
            "userAgent": USER_AGENT,
            // The session pass itself is not shown: it is a random number the server gave out at sign-in.
            "signedIn": token.is_some(),
            "raw": raw,
        });
        let ok = self.app.emit("net-ask", shown).is_ok() && rx.recv_timeout(Duration::from_secs(600)).unwrap_or(false);
        self.ask.pending.lock().unwrap().remove(&id);
        ok
    }
}

impl Wire for Asking {
    fn call(&self, path: &str, token: Option<&str>, body: Value) -> Result<Value, WireError> {
        let raw = body.to_string();
        if !self.approved(path, token, &raw) {
            return Err(WireError { status: 0, message: CANCELLED.into() });
        }
        self.inner.send(path, token, &raw)
    }
}

/// The answer to a `net-ask`. `trust`: don't ask again until the app closes.
#[tauri::command]
pub fn net_answer(ask: State<SharedAsk>, id: u64, ok: bool, trust: bool) {
    if ok && trust {
        ask.trusted.store(true, Ordering::SeqCst);
    }
    if let Some(tx) = ask.pending.lock().unwrap().remove(&id) {
        let _ = tx.send(ok);
    }
}

struct Session {
    username: String,
    account_id: String,
    token: Zeroizing<String>,
    key: Secret,
    /// The backup's keys as last listed, opened and checked.
    remote: Vec<Entry>,
}

pub struct Backup {
    wire: Box<dyn Wire>,
    session: Option<Session>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    entries: Vec<EntryView>,
    /// Boxes the server returned that this account's key does not open, or that failed a check.
    unreadable: usize,
}

fn aad(account_id: &str, role: &str) -> String {
    format!("vanityvault-backup/v1/{account_id}/{role}")
}

fn entry_id(key: &Secret, address: &str) -> String {
    hex(&derive(&key[..], &format!("vanityvault-backup/v1/entry-id/{address}"))[..8])
}

/// (`auth`, `kek`) from a password.
fn password_keys(password: &str, salt: &[u8], kdf: Kdf) -> Result<(Secret, Secret), String> {
    let master = vault::stretch(password, salt, kdf)?;
    Ok((derive(&master[..], "vanityvault-backup/v1/auth"), derive(&master[..], "vanityvault-backup/v1/kek")))
}

/// The same pair from the recovery key, which is random and long, so it needs no slow step.
fn recovery_keys(recovery: &[u8]) -> (Secret, Secret) {
    (derive(recovery, "vanityvault-backup/v1/recovery-auth"), derive(recovery, "vanityvault-backup/v1/recovery-kek"))
}

fn kdf_json(k: Kdf) -> Value {
    json!({ "m_kib": k.m_kib, "t": k.t, "p": k.p })
}

fn sealed_json(s: &Sealed) -> Value {
    json!({ "nonce": s.nonce, "ct": s.ct })
}

fn sealed_from(v: &Value) -> Option<Sealed> {
    Some(Sealed { nonce: v["nonce"].as_str()?.to_string(), ct: v["ct"].as_str()?.to_string() })
}

fn clean_username(username: &str) -> Result<String, String> {
    let u = username.trim().to_ascii_lowercase();
    // A made-up name, never an email address: the backup has no business knowing who anyone is.
    if u.contains('@') {
        return Err("Make up a username instead of using your email address. We don't want to know it.".into());
    }
    let ok = (6..=32).contains(&u.len())
        && u.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c))
        && u.as_bytes()[0].is_ascii_alphanumeric();
    ok.then_some(u).ok_or_else(|| "Usernames have 6 to 32 letters, digits, dots, dashes or underscores.".into())
}

fn strong(password: &str) -> Result<(), String> {
    vault::strength(password).ok.then_some(()).ok_or_else(|| "That password is too easy to guess. Use a longer phrase.".into())
}

impl Backup {
    pub fn new(wire: Box<dyn Wire>) -> Self {
        Self { wire, session: None }
    }

    pub fn username(&self) -> Option<&str> {
        self.session.as_ref().map(|s| s.username.as_str())
    }

    /// Forgets the session and every opened key, without telling the server.
    pub fn forget(&mut self) {
        self.session = None;
    }

    fn open_call(&self, path: &str, body: Value) -> Result<Value, String> {
        self.wire.call(path, None, body).map_err(|e| e.message)
    }

    /// A call as the signed-in user. If the server no longer knows the session, the app signs out.
    fn call(&mut self, path: &str, body: Value) -> Result<Value, String> {
        let s = self.session.as_ref().ok_or("Sign in to your backup first.")?;
        self.wire.call(path, Some(&s.token), body).map_err(|e| {
            if e.status == 401 {
                self.session = None;
            }
            e.message
        })
    }

    fn start(&mut self, username: String, account_id: String, key: Secret, answer: &Value) -> Result<(), String> {
        let token = Zeroizing::new(answer["token"].as_str().ok_or(BAD_ANSWER)?.to_string());
        self.session = Some(Session { username, account_id, token, key, remote: Vec::new() });
        Ok(())
    }

    /// The salt and password settings for a username. The server answers for unknown names too.
    fn prelogin(&self, username: &str) -> Result<(Vec<u8>, Kdf), String> {
        let r = self.open_call("prelogin", json!({ "username": username }))?;
        let salt = r["salt"].as_str().and_then(|s| B64.decode(s).ok()).filter(|s| s.len() == 16).ok_or(BAD_ANSWER)?;
        let num = |name: &str| r["kdf"][name].as_u64().and_then(|n| u32::try_from(n).ok());
        let kdf = Kdf { m_kib: num("m_kib").ok_or(BAD_ANSWER)?, t: num("t").ok_or(BAD_ANSWER)?, p: num("p").ok_or(BAD_ANSWER)? };
        let fine = (MIN_KDF.m_kib..=1_048_576).contains(&kdf.m_kib) && (MIN_KDF.t..=10).contains(&kdf.t) && (MIN_KDF.p..=4).contains(&kdf.p);
        fine.then_some((salt, kdf)).ok_or_else(|| "The VanityCloud server asked for password settings this app does not accept.".into())
    }

    /// Makes an account and returns its recovery key, which is shown once and never stored.
    pub fn register(&mut self, username: &str, password: &str, kdf: Kdf) -> Result<String, String> {
        let username = clean_username(username)?;
        strong(password)?;
        let account_id = hex(&random::<16>());
        let salt = random::<16>();
        let (auth, kek) = password_keys(password, &salt, kdf)?;
        let key: Secret = Zeroizing::new(random::<32>());
        let recovery = Zeroizing::new(random::<{ vault::RECOVERY_BYTES }>());
        let (recovery_auth, recovery_kek) = recovery_keys(&recovery[..]);
        let body = json!({
            "account_id": account_id,
            "username": username,
            "salt": B64.encode(salt),
            "kdf": kdf_json(kdf),
            "auth_key": B64.encode(&auth[..]),
            "wrapped_key": sealed_json(&seal(&kek, &aad(&account_id, "key/password"), &key[..])),
            "recovery_auth": B64.encode(&recovery_auth[..]),
            "wrapped_key_recovery": sealed_json(&seal(&recovery_kek, &aad(&account_id, "key/recovery"), &key[..])),
        });
        let r = self.open_call("register", body)?;
        self.start(username, account_id, key, &r)?;
        Ok(vault::recovery_text(&recovery))
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let username = clean_username(username)?;
        let (salt, kdf) = self.prelogin(&username)?;
        let (auth, kek) = password_keys(password, &salt, kdf)?;
        let r = self.open_call("login", json!({ "username": username, "auth_key": B64.encode(&auth[..]) }))?;
        let account_id = r["account_id"].as_str().ok_or(BAD_ANSWER)?.to_string();
        let key = sealed_from(&r["wrapped_key"])
            .and_then(|w| unseal(&kek, &aad(&account_id, "key/password"), &w))
            .and_then(|k| vault::to_secret(&k))
            .ok_or("The VanityCloud server sent a key that this password does not open.")?;
        self.start(username, account_id, key, &r)
    }

    /// Signs in with the recovery key and sets a new password. The recovery key stays valid.
    pub fn recover(&mut self, username: &str, recovery_key: &str, new_password: &str, kdf: Kdf) -> Result<(), String> {
        let username = clean_username(username)?;
        let recovery = vault::recovery_bytes(recovery_key).ok_or("That is not a valid recovery key. It has 32 letters and digits.")?;
        strong(new_password)?;
        let (recovery_auth, recovery_kek) = recovery_keys(&recovery[..]);
        let proof = B64.encode(&recovery_auth[..]);
        let r = self.open_call("recover/start", json!({ "username": username, "recovery_auth": proof }))?;
        let account_id = r["account_id"].as_str().ok_or(BAD_ANSWER)?.to_string();
        let key = sealed_from(&r["wrapped_key_recovery"])
            .and_then(|w| unseal(&recovery_kek, &aad(&account_id, "key/recovery"), &w))
            .and_then(|k| vault::to_secret(&k))
            .ok_or("The VanityCloud server sent a key that this recovery key does not open.")?;
        let salt = random::<16>();
        let (auth, kek) = password_keys(new_password, &salt, kdf)?;
        let body = json!({
            "username": username,
            "recovery_auth": proof,
            "salt": B64.encode(salt),
            "kdf": kdf_json(kdf),
            "auth_key": B64.encode(&auth[..]),
            "wrapped_key": sealed_json(&seal(&kek, &aad(&account_id, "key/password"), &key[..])),
        });
        let r = self.open_call("recover/finish", body)?;
        self.start(username, account_id, key, &r)
    }

    pub fn logout(&mut self) {
        let _ = self.call("logout", json!({}));
        self.session = None;
    }

    /// The sign-in proof for the current password, to confirm a change that can't be undone.
    fn prove(&self, password: &str) -> Result<String, String> {
        let username = self.username().ok_or("Sign in to your backup first.")?.to_string();
        let (salt, kdf) = self.prelogin(&username)?;
        Ok(B64.encode(&password_keys(password, &salt, kdf)?.0[..]))
    }

    pub fn change_password(&mut self, current: &str, new_password: &str, kdf: Kdf) -> Result<(), String> {
        strong(new_password)?;
        let proof = self.prove(current)?;
        let s = self.session.as_ref().ok_or("Sign in to your backup first.")?;
        let salt = random::<16>();
        let (auth, kek) = password_keys(new_password, &salt, kdf)?;
        let body = json!({
            "auth_key": proof,
            "salt": B64.encode(salt),
            "kdf": kdf_json(kdf),
            "new_auth_key": B64.encode(&auth[..]),
            "wrapped_key": sealed_json(&seal(&kek, &aad(&s.account_id, "key/password"), &s.key[..])),
        });
        let r = self.call("password", body)?;
        let token = Zeroizing::new(r["token"].as_str().ok_or(BAD_ANSWER)?.to_string());
        if let Some(s) = self.session.as_mut() {
            s.token = token;
        }
        Ok(())
    }

    /// Deletes the account and everything in it from the server.
    pub fn delete_account(&mut self, password: &str) -> Result<(), String> {
        let proof = self.prove(password)?;
        self.call("account/delete", json!({ "auth_key": proof }))?;
        self.session = None;
        Ok(())
    }

    /// Fetches every box, opens it and checks it. Boxes that fail are counted, not trusted.
    pub fn list(&mut self) -> Result<Listing, String> {
        let (account_id, key) = self.session.as_ref().map(|s| (s.account_id.clone(), s.key.clone())).ok_or("Sign in to your backup first.")?;
        let (mut found, mut seen, mut unreadable) = (Vec::new(), HashSet::new(), 0);
        let mut after = String::new();
        for _ in 0..MAX_PAGES {
            let r = self.call("entries/list", json!({ "after": after }))?;
            let page = r["entries"].as_array().ok_or(BAD_ANSWER)?;
            for e in page {
                let id = e["id"].as_str().ok_or(BAD_ANSWER)?;
                // The pages must move forward, or a server could keep the app asking forever.
                if id <= after.as_str() {
                    return Err(BAD_ANSWER.into());
                }
                after = id.to_string();
                let entry = sealed_from(e)
                    .and_then(|s| unseal(&key, &aad(&account_id, &format!("entry/{id}")), &s))
                    .and_then(|plain| Entry::from_plain(&plain))
                    .filter(|entry| entry.is_consistent() && entry_id(&key, &entry.view.address) == id);
                match entry {
                    Some(entry) if seen.insert(entry.view.address.clone()) => found.push(entry),
                    _ => unreadable += 1,
                }
            }
            if page.is_empty() || r["more"].as_bool() != Some(true) {
                break;
            }
        }
        found.sort_by(|a, b| b.view.at.cmp(&a.view.at));
        let entries = found.iter().map(|e| e.view.clone()).collect();
        if let Some(s) = self.session.as_mut() {
            s.remote = found;
        }
        Ok(Listing { entries, unreadable })
    }

    /// Locks these keys and stores them. A key that is already there is overwritten in place.
    pub fn upload(&mut self, entries: Vec<Entry>) -> Result<usize, String> {
        let (account_id, key) = self.session.as_ref().map(|s| (s.account_id.clone(), s.key.clone())).ok_or("Sign in to your backup first.")?;
        for chunk in entries.chunks(PER_CALL) {
            let boxes: Vec<Value> = chunk
                .iter()
                .map(|e| {
                    let id = entry_id(&key, &e.view.address);
                    let s = seal(&key, &aad(&account_id, &format!("entry/{id}")), &e.plain());
                    json!({ "id": id, "nonce": s.nonce, "ct": s.ct })
                })
                .collect();
            self.call("entries/put", json!({ "entries": boxes }))?;
        }
        let n = entries.len();
        if let Some(s) = self.session.as_mut() {
            for e in entries {
                if !s.remote.iter().any(|r| r.view.address == e.view.address) {
                    s.remote.push(e);
                }
            }
        }
        Ok(n)
    }

    /// The addresses in the backup as last listed or uploaded.
    fn backed_up(&self) -> Result<HashSet<String>, String> {
        let s = self.session.as_ref().ok_or("Sign in to your backup first.")?;
        Ok(s.remote.iter().map(|e| e.view.address.clone()).collect())
    }

    /// Copies of keys from the last listing, to put into the local vault.
    pub fn copies(&self, addresses: &[String]) -> Result<Vec<Entry>, String> {
        let s = self.session.as_ref().ok_or("Sign in to your backup first.")?;
        addresses
            .iter()
            .map(|a| s.remote.iter().find(|e| &e.view.address == a).map(Entry::copy).ok_or_else(|| "That key is not in the backup.".to_string()))
            .collect()
    }

    /// Removes keys from the backup. The local vault is not touched.
    pub fn remove(&mut self, addresses: &[String]) -> Result<usize, String> {
        let key = self.session.as_ref().map(|s| s.key.clone()).ok_or("Sign in to your backup first.")?;
        for chunk in addresses.chunks(PER_CALL) {
            let ids: Vec<String> = chunk.iter().map(|a| entry_id(&key, a)).collect();
            self.call("entries/delete", json!({ "ids": ids }))?;
        }
        if let Some(s) = self.session.as_mut() {
            s.remote.retain(|e| !addresses.contains(&e.view.address));
        }
        Ok(addresses.len())
    }
}

// --- the commands the UI can call. Each one that can take a while stays off the UI thread.

pub type SharedBackup = Mutex<Backup>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    signed_in: bool,
    username: String,
}

#[tauri::command]
pub fn backup_status(backup: State<SharedBackup>) -> Status {
    let b = backup.lock().unwrap();
    Status { signed_in: b.username().is_some(), username: b.username().unwrap_or_default().to_string() }
}

#[tauri::command(async)]
pub fn backup_register(backup: State<SharedBackup>, username: String, password: String) -> Result<String, String> {
    backup.lock().unwrap().register(&username, &password, vault::DEFAULT_KDF)
}

#[tauri::command(async)]
pub fn backup_login(backup: State<SharedBackup>, username: String, password: String) -> Result<(), String> {
    backup.lock().unwrap().login(&username, &password)
}

#[tauri::command(async)]
pub fn backup_recover(backup: State<SharedBackup>, username: String, recovery_key: String, password: String) -> Result<(), String> {
    backup.lock().unwrap().recover(&username, &recovery_key, &password, vault::DEFAULT_KDF)
}

#[tauri::command(async)]
pub fn backup_logout(backup: State<SharedBackup>) {
    backup.lock().unwrap().logout();
}

#[tauri::command(async)]
pub fn backup_list(backup: State<SharedBackup>) -> Result<Listing, String> {
    backup.lock().unwrap().list()
}

/// Backs up keys from the local vault.
#[tauri::command(async)]
pub fn backup_upload(backup: State<SharedBackup>, vault: State<SharedVault>, addresses: Vec<String>) -> Result<usize, String> {
    let entries = vault.lock().unwrap().copies(&addresses)?;
    backup.lock().unwrap().upload(entries)
}

/// Backs up the vault's keys that the backup doesn't have yet, for "back up new finds
/// automatically". Returns how many were sent (0 sends nothing and asks nothing).
#[tauri::command(async)]
pub fn backup_upload_new(backup: State<SharedBackup>, vault: State<SharedVault>) -> Result<usize, String> {
    let have = backup.lock().unwrap().backed_up()?;
    let missing: Vec<String> = vault.lock().unwrap().list().into_iter().map(|e| e.address).filter(|a| !have.contains(a)).collect();
    if missing.is_empty() {
        return Ok(0);
    }
    let entries = vault.lock().unwrap().copies(&missing)?;
    backup.lock().unwrap().upload(entries)
}

/// Puts keys from the backup into the local vault. Returns how many were new to it.
#[tauri::command(async)]
pub fn backup_restore(backup: State<SharedBackup>, vault: State<SharedVault>, addresses: Vec<String>) -> Result<usize, String> {
    let entries = backup.lock().unwrap().copies(&addresses)?;
    vault.lock().unwrap().add_many(entries)
}

#[tauri::command(async)]
pub fn backup_remove(backup: State<SharedBackup>, addresses: Vec<String>) -> Result<usize, String> {
    backup.lock().unwrap().remove(&addresses)
}

#[tauri::command(async)]
pub fn backup_password(backup: State<SharedBackup>, current: String, password: String) -> Result<(), String> {
    backup.lock().unwrap().change_password(&current, &password, vault::DEFAULT_KDF)
}

#[tauri::command(async)]
pub fn backup_delete_account(backup: State<SharedBackup>, password: String) -> Result<(), String> {
    backup.lock().unwrap().delete_account(&password)
}
