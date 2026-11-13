//! The hunt and the commands the UI can call.
//!
//! Private keys never leave the core unless the user asks to reveal or export one: the UI only
//! receives addresses, counters and status. A hunt runs on the graphics card when one passed its
//! startup check (see `gpu.rs`), otherwise on the processor. Either way every key is built from
//! the operating system's random source, and every match is derived again on the CPU and checked
//! by a second route before it is kept.
//!
//! A hunt looks for the main pattern, for the words of the hunt list (see `list.rs`), or both.
//! Finding the main pattern ends the hunt. A hunt list find is kept and the hunt carries on.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use zeroize::Zeroizing;

use crate::gpu::Gpu;
use crate::list::{Packed, Words};
use crate::pattern::Pattern;
use crate::vault::{self, Entry, EntryView, Strength, Vault};

pub type SharedVault = Arc<Mutex<Vault>>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Hit {
    #[serde(flatten)]
    entry: EntryView,
    /// False when the key could not be written to the vault file; `error` says why.
    saved: bool,
    error: String,
    /// Hunt list finds so far in this hunt.
    finds: u64,
}

/// What the UI shows before and during a hunt. A zero means that part is not in play.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    /// Average tries until the main pattern is found.
    mean: f64,
    /// Number of words in the hunt list, and the average tries between its finds.
    words: usize,
    list_mean: f64,
}

/// The main pattern and the hunt list as typed. At least one of them must be there.
fn plan(start: &str, end: &str, any_case: bool, list: &str) -> Result<(Option<Pattern>, Option<Words>), String> {
    let words = Words::parse(list, any_case)?;
    let words = (words.len() > 0).then_some(words);
    if start.is_empty() && end.is_empty() && words.is_some() {
        return Ok((None, words));
    }
    Ok((Some(Pattern::parse(start, end, any_case)?), words))
}

fn estimate(pat: &Option<Pattern>, words: &Option<Words>) -> Estimate {
    Estimate {
        mean: pat.as_ref().map_or(0.0, Pattern::mean_tries),
        words: words.as_ref().map_or(0, Words::len),
        list_mean: words.as_ref().map_or(0.0, Words::mean_tries),
    }
}

/// The hunt list while a hunt runs. `version` goes up whenever a word is ticked off, which tells
/// the engines to pick up the new lookup table.
struct List {
    state: Mutex<(Words, Arc<Packed>)>,
    version: AtomicU64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Tick {
    tried: u64,
    rate: f64,
    elapsed_ms: u64,
    /// True while the graphics card is doing the work.
    gpu: bool,
}

/// The graphics card as the UI sees it: "starting" while it is being set up and checked,
/// "ready" when hunts will use it, "none" when this PC hunts on the processor.
#[derive(Clone, Serialize)]
pub struct Card {
    state: &'static str,
    name: String,
}

#[derive(Clone, Serialize)]
pub struct About {
    threads: usize,
    card: Card,
}

pub struct Engine {
    run: Mutex<Option<Arc<AtomicBool>>>,
    gpu: Arc<Mutex<Option<Gpu>>>,
    card: Arc<Mutex<Card>>,
}

impl Default for Engine {
    fn default() -> Self {
        let card = Card { state: "starting", name: String::new() };
        Self { run: Mutex::default(), gpu: Arc::default(), card: Arc::new(Mutex::new(card)) }
    }
}

impl Engine {
    fn stop(&self) {
        if let Some(run) = self.run.lock().unwrap().take() {
            run.store(false, Ordering::SeqCst);
        }
    }

    /// Sets up the graphics card in the background, so the window opens right away.
    pub fn warm_up(&self, app: AppHandle) {
        let (gpu, card) = (self.gpu.clone(), self.card.clone());
        std::thread::spawn(move || {
            // A driver that crashes instead of reporting an error must not take the app with it.
            let opened = std::panic::catch_unwind(Gpu::open).unwrap_or_else(|_| Err("the graphics driver crashed".into()));
            let now = match opened {
                Ok(g) => {
                    let name = g.name.clone();
                    *gpu.lock().unwrap() = Some(g);
                    Card { state: "ready", name }
                }
                Err(_) => Card { state: "none", name: String::new() },
            };
            *card.lock().unwrap() = now.clone();
            let _ = app.emit("engine-about", About { threads: threads(), card: now });
        });
    }
}

fn threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).saturating_sub(1).max(1)
}

/// Writes the base58 address of `seed`'s public key into `out` and returns its length.
fn address_of(seed: &[u8; 32], out: &mut [u8; 44]) -> usize {
    let public = SigningKey::from_bytes(seed).verifying_key().to_bytes();
    bs58::encode(public).onto(&mut out[..]).expect("a 32-byte key always fits in 44 characters")
}

/// Second check, by a different route: decode the address back to bytes and compare it with a
/// public key derived again from the seed.
fn verify(seed: &[u8; 32], address: &str) -> bool {
    let mut decoded = [0u8; 32];
    match bs58::decode(address).onto(&mut decoded[..]) {
        Ok(32) => decoded == SigningKey::from_bytes(seed).verifying_key().to_bytes(),
        _ => false,
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Everything the threads of one hunt share.
#[derive(Clone)]
struct Hunt {
    pat: Option<Arc<Pattern>>,
    list: Option<Arc<List>>,
    finds: Arc<AtomicU64>,
    run: Arc<AtomicBool>,
    tried: Arc<AtomicU64>,
    on_gpu: Arc<AtomicBool>,
    vault: SharedVault,
    app: AppHandle,
}

impl Hunt {
    /// Decides what to do with a key that an engine's quick check let through. The address is
    /// derived here, checked by the second route, and compared in full with the main pattern and
    /// the hunt list. False means the key fits nothing, so the quick check was wrong.
    fn candidate(&self, seed: Zeroizing<[u8; 32]>) -> bool {
        let mut buf = [0u8; 44];
        let n = address_of(&seed, &mut buf);
        let address = String::from_utf8_lossy(&buf[..n]).into_owned();
        if !verify(&seed, &address) {
            return false;
        }
        if let Some(pat) = self.pat.as_ref().filter(|p| p.matches(&buf[..n])) {
            // The main pattern: the first key ends the hunt.
            if self.run.swap(false, Ordering::SeqCst) {
                self.keep("hunt-hit", address, seed, pat.start.len(), pat.end.len());
            }
            return true;
        }
        let Some(list) = &self.list else { return false };
        let mut state = list.state.lock().unwrap();
        let Some(taken) = state.0.take(&buf[..n]) else { return false };
        if taken.fresh {
            state.1 = Arc::new(state.0.pack());
            list.version.fetch_add(1, Ordering::SeqCst);
            let finished = self.pat.is_none() && state.0.done();
            drop(state);
            self.finds.fetch_add(1, Ordering::Relaxed);
            self.keep("hunt-find", address, seed, taken.start_len, taken.end_len);
            // Nothing left to look for.
            if finished && self.run.swap(false, Ordering::SeqCst) {
                let _ = self.app.emit("hunt-end", self.finds.load(Ordering::Relaxed));
            }
        }
        true
    }

    /// Puts the key into the vault and tells the UI.
    fn keep(&self, event: &str, address: String, seed: Zeroizing<[u8; 32]>, prefix_len: usize, suffix_len: usize) {
        let view = EntryView { address, prefix_len, suffix_len, tries: self.tried.load(Ordering::Relaxed), at: now_ms() };
        let saved = self.vault.lock().unwrap().add(Entry { view: view.clone(), seed });
        let finds = self.finds.load(Ordering::Relaxed);
        let hit = Hit { entry: view, saved: saved.is_ok(), error: saved.err().unwrap_or_default(), finds };
        let _ = self.app.emit(event, hit);
    }

    /// The hunt list's current lookup table, if it changed since `seen`.
    fn newer_table(&self, seen: &mut u64) -> Option<Arc<Packed>> {
        let list = self.list.as_ref()?;
        let version = list.version.load(Ordering::SeqCst);
        if version == *seen {
            return None;
        }
        *seen = version;
        Some(list.state.lock().unwrap().1.clone())
    }

    fn on_cpu(&self) {
        self.on_gpu.store(false, Ordering::Relaxed);
        for _ in 0..threads() {
            let hunt = self.clone();
            std::thread::spawn(move || hunt.cpu_worker());
        }
    }

    /// One fresh draw from the operating system's random source per key.
    fn cpu_worker(self) {
        let (mut seen, mut table) = (u64::MAX, None);
        let mut buf = [0u8; 44];
        while self.run.load(Ordering::Relaxed) && !self.on_gpu.load(Ordering::Relaxed) {
            if let Some(newer) = self.newer_table(&mut seen) {
                table = Some(newer);
            }
            for _ in 0..64 {
                let mut seed = Zeroizing::new([0u8; 32]);
                OsRng.fill_bytes(&mut seed[..]);
                self.tried.fetch_add(1, Ordering::Relaxed);
                let n = address_of(&seed, &mut buf);
                let addr = &buf[..n];
                if self.pat.as_ref().is_some_and(|p| p.matches(addr)) || table.as_ref().is_some_and(|t| t.hit(addr)) {
                    self.candidate(seed);
                    break;
                }
            }
        }
    }

    /// Runs batches on the graphics card. If the card fails, or reports a match the CPU does not
    /// agree with, it is dropped until the app restarts and the hunt carries on with the processor.
    fn on_card(self, gpu: Arc<Mutex<Option<Gpu>>>, card: Arc<Mutex<Card>>) {
        let mut slot = gpu.lock().unwrap();
        let healthy = match slot.as_mut() {
            Some(g) => self.card_loop(g),
            None => false,
        };
        if healthy {
            return;
        }
        *slot = None;
        drop(slot);
        let off = Card { state: "none", name: String::new() };
        *card.lock().unwrap() = off.clone();
        let _ = self.app.emit("engine-about", About { threads: threads(), card: off });
        self.on_cpu();
    }

    /// For a hunt that began on the processor while the graphics card was still being set up:
    /// hands the hunt over as soon as the card is ready. The processor threads stop by themselves.
    fn move_to_card_when_ready(self, gpu: Arc<Mutex<Option<Gpu>>>, card: Arc<Mutex<Card>>) {
        while self.run.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(500));
            let state = card.lock().unwrap().state;
            if state == "ready" {
                self.on_gpu.store(true, Ordering::Relaxed);
                return self.on_card(gpu, card);
            }
            if state == "none" {
                return;
            }
        }
    }

    /// False means the card can no longer be trusted.
    fn card_loop(&self, gpu: &mut Gpu) -> bool {
        if gpu.set_pattern(self.pat.as_deref()).is_err() || gpu.set_list(None).is_err() {
            return false;
        }
        let mut seen = u64::MAX;
        while self.run.load(Ordering::Relaxed) {
            if let Some(table) = self.newer_table(&mut seen) {
                if gpu.set_list(Some(&table)).is_err() {
                    return false;
                }
            }
            let Ok((n, seeds)) = gpu.batch() else { return false };
            self.tried.fetch_add(n, Ordering::Relaxed);
            // A batch can report a word twice before the card has the updated table; `candidate`
            // keeps the first and passes over the rest. A report that fits nothing is a fault.
            if !seeds.into_iter().all(|seed| self.candidate(seed)) {
                return false;
            }
        }
        true
    }

    /// Reports progress four times a second. The speed is an average over the last two seconds,
    /// which stays steady even though the graphics card counts in big batches.
    fn reporter(self) {
        let t0 = Instant::now();
        let mut window = VecDeque::from([(0u64, t0)]);
        loop {
            std::thread::sleep(Duration::from_millis(250));
            if !self.run.load(Ordering::Relaxed) {
                break;
            }
            let (now, now_t) = (self.tried.load(Ordering::Relaxed), Instant::now());
            if window.len() == 8 {
                window.pop_front();
            }
            let (then, then_t) = window[0];
            window.push_back((now, now_t));
            let tick = Tick {
                tried: now,
                rate: (now - then) as f64 / (now_t - then_t).as_secs_f64(),
                elapsed_ms: t0.elapsed().as_millis() as u64,
                gpu: self.on_gpu.load(Ordering::Relaxed),
            };
            let _ = self.app.emit("hunt-tick", tick);
        }
    }
}

#[tauri::command]
pub fn about(engine: State<Engine>) -> About {
    About { threads: threads(), card: engine.card.lock().unwrap().clone() }
}

#[tauri::command]
pub fn hunt_estimate(start: String, end: String, any_case: bool, list: String) -> Result<Estimate, String> {
    let (pat, words) = plan(&start, &end, any_case, &list)?;
    Ok(estimate(&pat, &words))
}

#[tauri::command]
pub fn hunt_start(
    app: AppHandle,
    engine: State<Engine>,
    vault: State<SharedVault>,
    start: String,
    end: String,
    any_case: bool,
    list: String,
) -> Result<Estimate, String> {
    let (pat, words) = plan(&start, &end, any_case, &list)?;
    if vault.lock().unwrap().status() != "open" {
        return Err("Unlock the vault first, so there is somewhere safe to put the key.".into());
    }
    let numbers = estimate(&pat, &words);
    let run = Arc::new(AtomicBool::new(true));
    if let Some(old) = engine.run.lock().unwrap().replace(run.clone()) {
        old.store(false, Ordering::SeqCst);
    }
    let state = engine.card.lock().unwrap().state;
    let on_gpu = state == "ready";
    let hunt = Hunt {
        pat: pat.map(Arc::new),
        list: words.map(|w| {
            let table = Arc::new(w.pack());
            Arc::new(List { state: Mutex::new((w, table)), version: AtomicU64::new(0) })
        }),
        finds: Arc::default(),
        run,
        tried: Arc::default(),
        on_gpu: Arc::new(AtomicBool::new(on_gpu)),
        vault: vault.inner().clone(),
        app,
    };
    if on_gpu {
        let (hunt, gpu, card) = (hunt.clone(), engine.gpu.clone(), engine.card.clone());
        std::thread::spawn(move || hunt.on_card(gpu, card));
    } else {
        hunt.on_cpu();
        if state == "starting" {
            let (hunt, gpu, card) = (hunt.clone(), engine.gpu.clone(), engine.card.clone());
            std::thread::spawn(move || hunt.move_to_card_when_ready(gpu, card));
        }
    }
    std::thread::spawn(move || hunt.reporter());
    Ok(numbers)
}

#[tauri::command]
pub fn hunt_stop(engine: State<Engine>) {
    engine.stop();
}

#[tauri::command]
pub fn vault_status(vault: State<SharedVault>) -> &'static str {
    vault.lock().unwrap().status()
}

#[tauri::command]
pub fn password_check(password: String) -> Strength {
    vault::strength(&password)
}

// The three commands below run the slow password step, so they stay off the UI thread.

#[tauri::command(async)]
pub fn vault_create(vault: State<SharedVault>, password: String) -> Result<String, String> {
    vault.lock().unwrap().create(&password, vault::DEFAULT_KDF)
}

#[tauri::command(async)]
pub fn vault_unlock(vault: State<SharedVault>, password: String) -> Result<(), String> {
    vault.lock().unwrap().unlock(&password)
}

#[tauri::command(async)]
pub fn vault_recover(vault: State<SharedVault>, recovery_key: String, password: String) -> Result<(), String> {
    vault.lock().unwrap().recover(&recovery_key, &password, vault::DEFAULT_KDF)
}

/// Locking the vault also drops the backup session, which holds opened keys of its own.
#[tauri::command(async)]
pub fn vault_lock(engine: State<Engine>, vault: State<SharedVault>, backup: State<crate::backup::SharedBackup>) {
    engine.stop();
    vault.lock().unwrap().lock();
    backup.lock().unwrap().forget();
}

/// Checks the vault password without changing anything (the first step of "delete everything").
#[tauri::command(async)]
pub fn vault_check(vault: State<SharedVault>, password: String) -> Result<(), String> {
    vault.lock().unwrap().check(&password)
}

/// "Delete everything", the last step: stops any hunt, forgets the backup session and deletes
/// the vault file. The VanityCloud account is deleted before this, by the UI, through the backup.
#[tauri::command(async)]
pub fn vault_destroy(engine: State<Engine>, vault: State<SharedVault>, backup: State<crate::backup::SharedBackup>, password: String) -> Result<(), String> {
    vault.lock().unwrap().check(&password)?;
    engine.stop();
    backup.lock().unwrap().forget();
    vault.lock().unwrap().destroy(&password)
}

#[tauri::command]
pub fn vault_list(vault: State<SharedVault>) -> Vec<EntryView> {
    vault.lock().unwrap().list()
}

#[tauri::command]
pub fn vault_secret(vault: State<SharedVault>, address: String) -> Result<String, String> {
    vault.lock().unwrap().secret_base58(&address)
}

/// Saves the Solana keypair file into the user's Downloads folder and returns where it went.
#[tauri::command]
pub fn vault_export(app: AppHandle, vault: State<SharedVault>, address: String) -> Result<String, String> {
    let dir = app.path().download_dir().map_err(|_| "Could not find your Downloads folder.")?;
    Ok(vault.lock().unwrap().export_json(&address, &dir)?.display().to_string())
}

#[tauri::command]
pub fn vault_delete(vault: State<SharedVault>, address: String) -> Result<(), String> {
    vault.lock().unwrap().delete(&address)
}
