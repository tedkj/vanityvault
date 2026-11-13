//! The local vault: one encrypted file that holds every key the app finds.
//!
//! Scheme (the online backup in `backup.rs` uses the same building blocks):
//! - password -> Argon2id -> master key -> HKDF-SHA256 -> `kek`;
//! - a random 32-byte vault key, stored twice: wrapped with `kek`, and wrapped with a key derived
//!   from the printed recovery key;
//! - each entry is XChaCha20-Poly1305 under the vault key with a random 24-byte nonce.
//! The AAD ties every blob to this vault and to its role, so blobs can't be swapped around.
//! Nothing in the file is readable without the password or the recovery key, addresses included.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

const FORMAT: u32 = 1;
const MIN_SCORE: u8 = 3;
pub(crate) const RECOVERY_BYTES: usize = 20;
const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub(crate) type Secret = Zeroizing<[u8; 32]>;

/// Argon2id cost. The default takes about a second on a normal PC.
#[derive(Clone, Copy)]
pub struct Kdf {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}
pub const DEFAULT_KDF: Kdf = Kdf { m_kib: 262_144, t: 3, p: 1 };

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Sealed {
    pub(crate) nonce: String,
    pub(crate) ct: String,
}

#[derive(Serialize, Deserialize)]
struct KdfFile {
    alg: String,
    m_kib: u32,
    t: u32,
    p: u32,
    salt: String,
}

#[derive(Serialize, Deserialize)]
struct FileEntry {
    id: String,
    #[serde(flatten)]
    sealed: Sealed,
}

#[derive(Serialize, Deserialize)]
struct VaultFile {
    format: u32,
    vault_id: String,
    kdf: KdfFile,
    key_by_password: Sealed,
    key_by_recovery: Sealed,
    entries: Vec<FileEntry>,
}

/// What the UI may see of a key: everything except the secret.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryView {
    pub address: String,
    pub prefix_len: usize,
    pub suffix_len: usize,
    pub tries: u64,
    pub at: u64,
}

pub struct Entry {
    pub view: EntryView,
    pub seed: Secret,
}

struct Stored {
    id: String,
    entry: Entry,
}

struct Open {
    file: VaultFile,
    key: Secret,
    entries: Vec<Stored>,
}

pub struct Vault {
    path: PathBuf,
    open: Option<Open>,
}

#[derive(Serialize)]
pub struct Strength {
    score: u8,
    pub(crate) ok: bool,
    hint: String,
}

pub fn strength(password: &str) -> Strength {
    if password.is_empty() {
        return Strength { score: 0, ok: false, hint: String::new() };
    }
    let e = zxcvbn::zxcvbn(password, &[]);
    let score = u8::from(e.score());
    let hint = e.feedback().and_then(|f| f.warning()).map(|w| w.to_string()).unwrap_or_default();
    Strength { score, ok: score >= MIN_SCORE, hint }
}

pub(crate) fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    OsRng.fill_bytes(&mut b);
    b
}

pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn master_key(password: &str, kdf: &KdfFile) -> Result<Secret, String> {
    let salt = B64.decode(&kdf.salt).map_err(|_| "The vault file is damaged.")?;
    stretch(password, &salt, Kdf { m_kib: kdf.m_kib, t: kdf.t, p: kdf.p })
}

/// The slow password step (Argon2id): turns a password and a salt into a master key.
pub(crate) fn stretch(password: &str, salt: &[u8], kdf: Kdf) -> Result<Secret, String> {
    let params = Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32)).map_err(|_| "The password settings are not valid.")?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut out[..])
        .map_err(|_| "This PC could not run the password step (not enough free memory?).")?;
    Ok(out)
}

pub(crate) fn derive(ikm: &[u8], info: &str) -> Secret {
    let mut out = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(None, ikm).expand(info.as_bytes(), &mut out[..]).expect("32 bytes is a valid HKDF length");
    out
}

pub(crate) fn seal(key: &[u8; 32], aad: &str, plain: &[u8]) -> Sealed {
    let nonce = random::<24>();
    let ct = XChaCha20Poly1305::new(Key::from_slice(key))
        .encrypt(XNonce::from_slice(&nonce), Payload { msg: plain, aad: aad.as_bytes() })
        .expect("encrypting in memory cannot fail");
    Sealed { nonce: B64.encode(nonce), ct: B64.encode(ct) }
}

pub(crate) fn unseal(key: &[u8; 32], aad: &str, s: &Sealed) -> Option<Zeroizing<Vec<u8>>> {
    let nonce = B64.decode(&s.nonce).ok().filter(|n| n.len() == 24)?;
    let ct = B64.decode(&s.ct).ok()?;
    XChaCha20Poly1305::new(Key::from_slice(key))
        .decrypt(XNonce::from_slice(&nonce), Payload { msg: &ct, aad: aad.as_bytes() })
        .ok()
        .map(Zeroizing::new)
}

pub(crate) fn to_secret(bytes: &[u8]) -> Option<Secret> {
    let mut out = Zeroizing::new([0u8; 32]);
    if bytes.len() != 32 {
        return None;
    }
    out.copy_from_slice(bytes);
    Some(out)
}

pub(crate) fn recovery_text(bytes: &[u8; RECOVERY_BYTES]) -> String {
    // 20 bytes = 160 bits = 32 base32 characters, shown as 8 groups of 4.
    let (mut out, mut acc, mut bits) = (String::new(), 0u32, 0u32);
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            if !out.is_empty() && (out.len() + 1) % 5 == 0 {
                out.push('-');
            }
            out.push(B32[((acc >> bits) & 31) as usize] as char);
        }
    }
    out
}

pub(crate) fn recovery_bytes(text: &str) -> Option<Zeroizing<[u8; RECOVERY_BYTES]>> {
    let mut out = Zeroizing::new([0u8; RECOVERY_BYTES]);
    let (mut acc, mut bits, mut n) = (0u32, 0u32, 0usize);
    for c in text.chars().filter(|c| !c.is_whitespace() && *c != '-') {
        let v = B32.iter().position(|&a| a as char == c.to_ascii_uppercase())? as u32;
        acc = (acc << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            *out.get_mut(n)? = (acc >> bits) as u8;
            n += 1;
        }
    }
    (n == RECOVERY_BYTES).then_some(out)
}

fn aad(vault_id: &str, role: &str) -> String {
    format!("vanityvault/v{FORMAT}/{vault_id}/{role}")
}

impl Entry {
    /// The 64-byte Solana keypair: the secret seed followed by the public key.
    fn keypair(&self) -> Zeroizing<[u8; 64]> {
        let mut kp = Zeroizing::new([0u8; 64]);
        kp[..32].copy_from_slice(&self.seed[..]);
        kp[32..].copy_from_slice(&SigningKey::from_bytes(&self.seed).verifying_key().to_bytes());
        kp
    }

    /// Plaintext of an entry: the 32-byte seed, then the public details as JSON.
    pub(crate) fn plain(&self) -> Zeroizing<Vec<u8>> {
        let mut out = Zeroizing::new(self.seed.to_vec());
        out.extend_from_slice(&serde_json::to_vec(&self.view).expect("plain data always serializes"));
        out
    }

    pub(crate) fn from_plain(plain: &[u8]) -> Option<Self> {
        if plain.len() < 32 {
            return None;
        }
        Some(Self { seed: to_secret(&plain[..32])?, view: serde_json::from_slice(&plain[32..]).ok()? })
    }

    /// True when the secret really is the key of the address it is filed under.
    pub(crate) fn is_consistent(&self) -> bool {
        bs58::encode(SigningKey::from_bytes(&self.seed).verifying_key().to_bytes()).into_string() == self.view.address
    }

    pub(crate) fn copy(&self) -> Self {
        Self { view: self.view.clone(), seed: self.seed.clone() }
    }
}

impl Vault {
    pub fn new(path: PathBuf) -> Self {
        Self { path, open: None }
    }

    pub fn status(&self) -> &'static str {
        if self.open.is_some() {
            "open"
        } else if self.path.exists() {
            "locked"
        } else {
            "none"
        }
    }

    fn read(&self) -> Result<VaultFile, String> {
        let text = fs::read(&self.path).map_err(|e| format!("Could not read the vault file: {e}"))?;
        let file: VaultFile = serde_json::from_slice(&text).map_err(|_| "The vault file is damaged.")?;
        if file.format != FORMAT || file.kdf.alg != "argon2id" {
            return Err("This vault file was made by a newer version of VanityVault.".into());
        }
        Ok(file)
    }

    /// Writes to a temporary file first and then swaps it in, so a crash can't leave half a vault.
    fn write(path: &Path, file: &VaultFile) -> Result<(), String> {
        let err = |e: std::io::Error| format!("Could not save the vault file: {e}");
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(err)?;
        }
        let tmp = path.with_extension("tmp");
        let mut f = fs::File::create(&tmp).map_err(err)?;
        f.write_all(&serde_json::to_vec_pretty(file).expect("the vault file always serializes")).map_err(err)?;
        f.sync_all().map_err(err)?;
        drop(f);
        fs::rename(&tmp, path).map_err(err)
    }

    fn load_entries(file: &VaultFile, key: &[u8; 32]) -> Result<Vec<Stored>, String> {
        file.entries
            .iter()
            .map(|fe| {
                unseal(key, &aad(&file.vault_id, &format!("entry/{}", fe.id)), &fe.sealed)
                    .and_then(|p| Entry::from_plain(&p))
                    .map(|entry| Stored { id: fe.id.clone(), entry })
                    .ok_or_else(|| "The vault file is damaged: one of its keys can't be read.".to_string())
            })
            .collect()
    }

    /// Makes a new vault and returns the recovery key, which is shown once and never stored.
    pub fn create(&mut self, password: &str, kdf: Kdf) -> Result<String, String> {
        if self.path.exists() {
            return Err("A vault already exists on this PC.".into());
        }
        if !strength(password).ok {
            return Err("That password is too easy to guess. Use a longer phrase.".into());
        }
        let vault_id = hex(&random::<8>());
        let kdf = KdfFile { alg: "argon2id".into(), m_kib: kdf.m_kib, t: kdf.t, p: kdf.p, salt: B64.encode(random::<16>()) };
        let kek = derive(&master_key(password, &kdf)?[..], "vanityvault/v1/kek");
        let key: Secret = Zeroizing::new(random::<32>());
        let recovery = Zeroizing::new(random::<RECOVERY_BYTES>());
        let file = VaultFile {
            format: FORMAT,
            key_by_password: seal(&kek, &aad(&vault_id, "key/password"), &key[..]),
            key_by_recovery: seal(&derive(&recovery[..], "vanityvault/v1/recovery"), &aad(&vault_id, "key/recovery"), &key[..]),
            vault_id,
            kdf,
            entries: Vec::new(),
        };
        Self::write(&self.path, &file)?;
        self.open = Some(Open { file, key, entries: Vec::new() });
        Ok(recovery_text(&recovery))
    }

    pub fn unlock(&mut self, password: &str) -> Result<(), String> {
        let file = self.read()?;
        let kek = derive(&master_key(password, &file.kdf)?[..], "vanityvault/v1/kek");
        let key = unseal(&kek, &aad(&file.vault_id, "key/password"), &file.key_by_password)
            .and_then(|k| to_secret(&k))
            .ok_or("Wrong password.")?;
        let entries = Self::load_entries(&file, &key)?;
        self.open = Some(Open { file, key, entries });
        Ok(())
    }

    /// Opens the vault with the recovery key and sets a new password. The recovery key stays valid.
    pub fn recover(&mut self, recovery_key: &str, new_password: &str, kdf: Kdf) -> Result<(), String> {
        let mut file = self.read()?;
        let recovery = recovery_bytes(recovery_key).ok_or("That is not a valid recovery key. It has 32 letters and digits.")?;
        let key = unseal(&derive(&recovery[..], "vanityvault/v1/recovery"), &aad(&file.vault_id, "key/recovery"), &file.key_by_recovery)
            .and_then(|k| to_secret(&k))
            .ok_or("That recovery key does not open this vault.")?;
        if !strength(new_password).ok {
            return Err("That password is too easy to guess. Use a longer phrase.".into());
        }
        let entries = Self::load_entries(&file, &key)?;
        file.kdf = KdfFile { alg: "argon2id".into(), m_kib: kdf.m_kib, t: kdf.t, p: kdf.p, salt: B64.encode(random::<16>()) };
        let kek = derive(&master_key(new_password, &file.kdf)?[..], "vanityvault/v1/kek");
        file.key_by_password = seal(&kek, &aad(&file.vault_id, "key/password"), &key[..]);
        Self::write(&self.path, &file)?;
        self.open = Some(Open { file, key, entries });
        Ok(())
    }

    /// True when `password` opens the vault file. Changes nothing.
    pub fn check(&self, password: &str) -> Result<(), String> {
        let file = self.read()?;
        let kek = derive(&master_key(password, &file.kdf)?[..], "vanityvault/v1/kek");
        unseal(&kek, &aad(&file.vault_id, "key/password"), &file.key_by_password).map(|_| ()).ok_or_else(|| "Wrong vault password.".into())
    }

    /// Deletes the vault file for good, after checking the password. Afterwards a new vault can
    /// be made.
    pub fn destroy(&mut self, password: &str) -> Result<(), String> {
        self.check(password)?;
        self.open = None;
        fs::remove_file(&self.path).map_err(|e| format!("Could not delete the vault file: {e}"))
    }

    /// Forgets the vault key and every decrypted secret.
    pub fn lock(&mut self) {
        self.open = None;
    }

    pub fn list(&self) -> Vec<EntryView> {
        self.open.iter().flat_map(|o| o.entries.iter().rev().map(|s| s.entry.view.clone())).collect()
    }

    pub fn add(&mut self, entry: Entry) -> Result<(), String> {
        let path = self.path.clone();
        let o = self.open.as_mut().ok_or("The vault is locked.")?;
        if o.entries.iter().any(|s| s.entry.view.address == entry.view.address) {
            return Ok(());
        }
        let id = hex(&random::<8>());
        let sealed = seal(&o.key, &aad(&o.file.vault_id, &format!("entry/{id}")), &entry.plain());
        o.file.entries.push(FileEntry { id: id.clone(), sealed });
        o.entries.push(Stored { id, entry });
        Self::write(&path, &o.file)
    }

    /// Adds several keys with one write of the file. Keys already in the vault are passed over.
    /// Returns how many were new.
    pub fn add_many(&mut self, entries: Vec<Entry>) -> Result<usize, String> {
        let path = self.path.clone();
        let o = self.open.as_mut().ok_or("The vault is locked.")?;
        let mut added = 0;
        for entry in entries {
            if o.entries.iter().any(|s| s.entry.view.address == entry.view.address) {
                continue;
            }
            let id = hex(&random::<8>());
            let sealed = seal(&o.key, &aad(&o.file.vault_id, &format!("entry/{id}")), &entry.plain());
            o.file.entries.push(FileEntry { id: id.clone(), sealed });
            o.entries.push(Stored { id, entry });
            added += 1;
        }
        if added > 0 {
            Self::write(&path, &o.file)?;
        }
        Ok(added)
    }

    /// Copies of the keys with these addresses, for the backup to lock and upload.
    pub(crate) fn copies(&self, addresses: &[String]) -> Result<Vec<Entry>, String> {
        addresses.iter().map(|a| self.find(a).map(Entry::copy)).collect()
    }

    fn find(&self, address: &str) -> Result<&Entry, String> {
        let o = self.open.as_ref().ok_or("The vault is locked.")?;
        o.entries.iter().find(|s| s.entry.view.address == address).map(|s| &s.entry).ok_or_else(|| "That key is not in the vault.".into())
    }

    /// The secret key as base58 text, the form Phantom and Solflare import.
    pub fn secret_base58(&self, address: &str) -> Result<String, String> {
        Ok(bs58::encode(&self.find(address)?.keypair()[..]).into_string())
    }

    /// Writes the standard Solana keypair file (64 numbers) into `dir`. Never overwrites.
    pub fn export_json(&self, address: &str, dir: &Path) -> Result<PathBuf, String> {
        let kp = self.find(address)?.keypair();
        let mut text = Zeroizing::new(String::with_capacity(260));
        text.push('[');
        for (i, b) in kp.iter().enumerate() {
            if i > 0 {
                text.push(',');
            }
            text.push_str(&b.to_string());
        }
        text.push(']');
        let path = dir.join(format!("{address}.json"));
        let mut f = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => format!("{} already exists.", path.display()),
            _ => format!("Could not write the key file: {e}"),
        })?;
        f.write_all(text.as_bytes()).map_err(|e| format!("Could not write the key file: {e}"))?;
        Ok(path)
    }

    pub fn delete(&mut self, address: &str) -> Result<(), String> {
        let path = self.path.clone();
        let o = self.open.as_mut().ok_or("The vault is locked.")?;
        let i = o.entries.iter().position(|s| s.entry.view.address == address).ok_or("That key is not in the vault.")?;
        let gone = o.entries.remove(i);
        o.file.entries.retain(|fe| fe.id != gone.id);
        Self::write(&path, &o.file)
    }
}
