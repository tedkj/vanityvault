// The UI's only door to the Rust core. A private key crosses it in one case only:
// when the user asks to see or export one. Otherwise the UI gets addresses, progress and status.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getVersion } from '@tauri-apps/api/app';

export type Tick = { tried: number; rate: number; elapsedMs: number; gpu: boolean };
export type Entry = { address: string; prefixLen: number; suffixLen: number; tries: number; at: number };
// `finds` counts the hunt list finds so far in the running hunt.
export type Hit = Entry & { saved: boolean; error: string; finds: number };
// A zero means that part is not in play: `mean` is for the main pattern, `listMean` for the hunt list.
export type Estimate = { mean: number; words: number; listMean: number };
// The graphics card: 'starting' while it is set up and checked, 'ready' when hunts use it, 'none' when this PC hunts on the processor.
export type Card = { state: 'starting' | 'ready' | 'none'; name: string };
export type About = { threads: number; card: Card };
export type Status = 'none' | 'locked' | 'open';
export type Strength = { score: number; ok: boolean; hint: string };
export type BackupStatus = { signedIn: boolean; username: string };
// `unreadable` counts boxes from the server that this account's key does not open or that failed a check.
export type BackupListing = { entries: Entry[]; unreadable: number };
// A request the core holds until the user says Send: `raw` is the exact text it will send.
export type NetAsk = { id: number; path: string; url: string; userAgent: string; signedIn: boolean; raw: string };

const inTauri = '__TAURI_INTERNALS__' in window;

function sub<T>(name: string, cb: (p: T) => void): () => void {
  let un = () => {};
  let dead = false;
  listen<T>(name, (e) => cb(e.payload)).then((u) => (dead ? u() : (un = u)));
  return () => {
    dead = true;
    un();
  };
}

const real = {
  about: () => invoke<About>('about'),
  onAbout: (cb: (a: About) => void) => sub('engine-about', cb),
  estimate: (start: string, end: string, anyCase: boolean, list: string) => invoke<Estimate>('hunt_estimate', { start, end, anyCase, list }),
  start: (start: string, end: string, anyCase: boolean, list: string) => invoke<Estimate>('hunt_start', { start, end, anyCase, list }),
  stop: () => invoke<void>('hunt_stop'),
  onTick: (cb: (t: Tick) => void) => sub('hunt-tick', cb),
  // The main pattern was found, which ends the hunt.
  onHit: (cb: (h: Hit) => void) => sub('hunt-hit', cb),
  // A hunt list word was found; the hunt carries on.
  onFind: (cb: (h: Hit) => void) => sub('hunt-find', cb),
  // A hunt with only a list has found every word.
  onEnd: (cb: (finds: number) => void) => sub('hunt-end', cb),

  status: () => invoke<Status>('vault_status'),
  passwordCheck: (password: string) => invoke<Strength>('password_check', { password }),
  create: (password: string) => invoke<string>('vault_create', { password }),
  unlock: (password: string) => invoke<void>('vault_unlock', { password }),
  recover: (recoveryKey: string, password: string) => invoke<void>('vault_recover', { recoveryKey, password }),
  // "Delete everything": check the vault password first, delete the vault file last.
  vaultCheck: (password: string) => invoke<void>('vault_check', { password }),
  vaultDestroy: (password: string) => invoke<void>('vault_destroy', { password }),
  lock: () => invoke<void>('vault_lock'),
  list: () => invoke<Entry[]>('vault_list'),
  secret: (address: string) => invoke<string>('vault_secret', { address }),
  exportFile: (address: string) => invoke<string>('vault_export', { address }),
  remove: (address: string) => invoke<void>('vault_delete', { address }),

  // The online backup. Passwords go to the core, which locks everything before it is sent.
  backupStatus: () => invoke<BackupStatus>('backup_status'),
  // Returns the recovery key, which is shown once.
  backupRegister: (username: string, password: string) => invoke<string>('backup_register', { username, password }),
  backupLogin: (username: string, password: string) => invoke<void>('backup_login', { username, password }),
  backupRecover: (username: string, recoveryKey: string, password: string) => invoke<void>('backup_recover', { username, recoveryKey, password }),
  backupLogout: () => invoke<void>('backup_logout'),
  backupList: () => invoke<BackupListing>('backup_list'),
  backupUpload: (addresses: string[]) => invoke<number>('backup_upload', { addresses }),
  // Backs up whatever the vault has that the backup doesn't (for automatic backup of new finds).
  backupUploadNew: () => invoke<number>('backup_upload_new'),
  backupRestore: (addresses: string[]) => invoke<number>('backup_restore', { addresses }),
  backupRemove: (addresses: string[]) => invoke<number>('backup_remove', { addresses }),
  backupPassword: (current: string, password: string) => invoke<void>('backup_password', { current, password }),
  backupDeleteAccount: (password: string) => invoke<void>('backup_delete_account', { password }),
  // Nothing goes online until the user answers. `trust`: don't ask again until the app closes.
  onNetAsk: (cb: (a: NetAsk) => void) => sub('net-ask', cb),
  netAnswer: (id: number, ok: boolean, trust: boolean) => invoke<void>('net_answer', { id, ok, trust }),
  // A secret to the clipboard, kept out of Windows clipboard history and cleared after 30 seconds.
  copyPrivate: (text: string) => invoke<void>('copy_private', { text }),
};

// Browser-only stand-in so the UI can be worked on with `npm run dev`.
// It makes nothing real: the addresses are random characters and nothing is stored.
function mock(): typeof real {
  const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  const ticks = new Set<(t: Tick) => void>();
  const hitCbs = new Set<(h: Hit) => void>();
  const findCbs = new Set<(h: Hit) => void>();
  let entries: Entry[] = [];
  let remote: Entry[] = [];
  let user = '';
  let status: Status = 'none';
  // The stand-in for the core's asking wire: the same questions, with made-up values.
  const askCbs = new Set<(a: NetAsk) => void>();
  const answers = new Map<number, (ok: boolean) => void>();
  let asked = 0;
  let trusted = false;
  const fake = (n: number) => btoa(String.fromCharCode(...Array.from({ length: n }, () => Math.floor(Math.random() * 256))));
  const ask = async (path: string, body: Record<string, unknown>, signedIn = false) => {
    if (trusted) return;
    const id = ++asked;
    const ok = await new Promise<boolean>((r) => {
      answers.set(id, r);
      askCbs.forEach((cb) => cb({ id, path, url: `https://api.vanityvault.no/v1/${path}`, userAgent: 'VanityVault/preview', signedIn, raw: JSON.stringify(body) }));
    });
    if (!ok) throw 'Cancelled. Nothing was sent.';
  };
  const box = () => ({ nonce: fake(24), ct: fake(48) });
  let timer = 0;
  let ready = false;
  const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));
  const est = (s: string, e: string, list: string): Estimate => {
    const words = list.split(/[\s,;]+/).filter(Boolean);
    if (!s && !e && !words.length) throw 'Type at least one character.';
    for (const c of s + e + words.join('')) if (!B58.includes(c)) throw `"${c}" can't appear in a Solana address.`;
    const mean = s || e ? Math.pow(58, e.length) * (s ? 999.3 * Math.pow(58, s.length - 1) : 1) : 0;
    const p = words.reduce((sum, w) => sum + Math.pow(58, -w.length), 0);
    return { mean, words: words.length, listMean: p ? 1 / p : 0 };
  };
  const strength = (p: string): Strength => {
    const score = Math.min(4, Math.floor(p.length / 5));
    return { score, ok: score >= 3, hint: score < 3 && p ? 'Short passwords are easy to guess.' : '' };
  };
  return {
    about: async () => ({ threads: 15, card: { state: 'starting', name: '' } }),
    onAbout: (cb) => {
      const id = setTimeout(() => ((ready = true), cb({ threads: 15, card: { state: 'ready', name: 'Preview card (no real keys)' } })), 12000);
      return () => clearTimeout(id);
    },
    estimate: async (s, e, _c, list) => est(s, e, list),
    start: async (s, e, _c, list) => {
      const numbers = est(s, e, list);
      const mean = numbers.mean || Infinity;
      const words = list.split(/[\s,;]+/).filter(Boolean);
      let finds = 0;
      let beat = 0;
      clearInterval(timer);
      let tried = 0;
      const t0 = performance.now();
      const goal = -Math.log(1 - Math.random()) * mean;
      timer = window.setInterval(() => {
        const rate = ready ? 131e6 + (Math.random() - 0.5) * 4e6 : 1.02e6 + (Math.random() - 0.5) * 6e4;
        tried += rate / 4;
        // Every couple of seconds the preview pretends a list word was found.
        if (words.length && ++beat % 7 === 0) {
          const w = words[finds % words.length];
          let mid = '';
          for (let i = 0; i < 44 - w.length; i++) mid += B58[(Math.random() * 58) | 0];
          const entry = { address: mid + w, prefixLen: 0, suffixLen: w.length, tries: Math.round(tried), at: Date.now() };
          entries = [entry, ...entries];
          finds++;
          findCbs.forEach((cb) => cb({ ...entry, saved: true, error: '', finds }));
        }
        if (tried < goal) return ticks.forEach((cb) => cb({ tried, rate, elapsedMs: performance.now() - t0, gpu: ready }));
        clearInterval(timer);
        let mid = '';
        for (let i = 0; i < 43 - s.length - e.length; i++) mid += B58[(Math.random() * 58) | 0];
        const entry = { address: s + mid + e, prefixLen: s.length, suffixLen: e.length, tries: Math.round(tried), at: Date.now() };
        entries = [entry, ...entries];
        hitCbs.forEach((cb) => cb({ ...entry, saved: true, error: '', finds }));
      }, 250);
      return numbers;
    },
    stop: async () => clearInterval(timer),
    onTick: (cb) => (ticks.add(cb), () => ticks.delete(cb)),
    onHit: (cb) => (hitCbs.add(cb), () => hitCbs.delete(cb)),
    onFind: (cb) => (findCbs.add(cb), () => findCbs.delete(cb)),
    onEnd: () => () => {},

    status: async () => status,
    passwordCheck: async (p) => strength(p),
    create: async (p) => {
      await pause(900);
      if (!strength(p).ok) throw 'That password is too easy to guess. Use a longer phrase.';
      status = 'open';
      return 'K7QM-2XWD-PREV-IEW0-NLY4-NOTA-REAL-KEY7';
    },
    unlock: async (p) => {
      await pause(900);
      if (p !== 'preview') throw 'Wrong password.';
      status = 'open';
    },
    recover: async () => {
      await pause(900);
      status = 'open';
    },
    lock: async () => {
      clearInterval(timer);
      status = 'locked';
      user = '';
    },
    list: async () => entries,
    secret: async () => 'PreviewOnly' + 'x'.repeat(76),
    exportFile: async (a) => `C:\\Users\\you\\Downloads\\${a}.json`,
    remove: async (a) => {
      entries = entries.filter((e) => e.address !== a);
    },

    backupStatus: async () => ({ signedIn: !!user, username: user }),
    backupRegister: async (u, p) => {
      await pause(900);
      if (!strength(p).ok) throw 'That password is too easy to guess. Use a longer phrase.';
      if (u.includes('@')) throw "Make up a username instead of using your email address. We don't want to know it.";
      if (u.trim().length < 6) throw 'Usernames have 6 to 32 letters, digits, dots, dashes or underscores.';
      const n = u.trim().toLowerCase();
      await ask('register', { account_id: 'e845b209b15a56c46191b43614eee051', username: n, salt: fake(16), kdf: { m_kib: 262144, t: 3, p: 1 }, auth_key: fake(32), wrapped_key: box(), recovery_auth: fake(32), wrapped_key_recovery: box() });
      user = n;
      remote = [];
      return 'B4CK-UP2X-PREV-IEW0-NLY4-NOTA-REAL-KEY7';
    },
    backupLogin: async (u, p) => {
      await pause(900);
      await ask('prelogin', { username: u.trim().toLowerCase() });
      await ask('login', { username: u.trim().toLowerCase(), auth_key: fake(32) });
      if (p !== 'preview') throw 'Wrong username or password.';
      user = u.trim().toLowerCase();
    },
    backupRecover: async (u) => {
      await pause(900);
      user = u.trim().toLowerCase();
    },
    backupLogout: async () => {
      user = '';
    },
    backupList: async () => (await pause(300), { entries: remote, unreadable: 0 }),
    backupUpload: async (as) => {
      await ask('entries/put', { entries: as.map(() => ({ id: fake(8), ...box() })) }, true);
      await pause(500);
      const add = entries.filter((e) => as.includes(e.address) && !remote.some((r) => r.address === e.address));
      remote = [...add, ...remote];
      return as.length;
    },
    backupUploadNew: async () => {
      const add = entries.filter((e) => !remote.some((r) => r.address === e.address));
      if (!add.length) return 0;
      await ask('entries/put', { entries: add.map(() => ({ id: fake(8), ...box() })) }, true);
      remote = [...add, ...remote];
      return add.length;
    },
    backupRestore: async (as) => {
      const add = remote.filter((r) => as.includes(r.address) && !entries.some((e) => e.address === r.address));
      entries = [...add, ...entries];
      return add.length;
    },
    backupRemove: async (as) => {
      remote = remote.filter((r) => !as.includes(r.address));
      return as.length;
    },
    backupPassword: async () => {
      await pause(900);
    },
    backupDeleteAccount: async () => {
      await pause(600);
      user = '';
      remote = [];
    },
    copyPrivate: (text) => navigator.clipboard.writeText(text),
    vaultCheck: async () => {
      await pause(400);
    },
    vaultDestroy: async () => {
      await pause(300);
      entries = [];
      remote = [];
      user = '';
      status = 'none';
    },
    onNetAsk: (cb) => (askCbs.add(cb), () => askCbs.delete(cb)),
    netAnswer: async (id, ok, trust) => {
      if (ok && trust) trusted = true;
      answers.get(id)?.(ok);
      answers.delete(id);
    },
  };
}

export const engine = inTauri ? real : mock();

export const appVersion = () => (inTauri ? getVersion() : Promise.resolve('preview'));

export const win = {
  minimize: () => inTauri && getCurrentWindow().minimize(),
  toggleMaximize: () => inTauri && getCurrentWindow().toggleMaximize(),
  close: () => inTauri && getCurrentWindow().close(),
};
