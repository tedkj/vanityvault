// VanityCloud as the screens see it: the account and the keys in it. The Vault lists and acts on
// the keys, the Backup screen runs the account, and both read this one copy.
import { engine, type Entry } from './engine';

// Keys per upload, as PER_CALL in backup.rs: each batch is its own Send window.
export const PER_SEND = 200;

export const cloud = $state({
  // null until the core has said whether a session is open.
  signedIn: null as boolean | null,
  username: '',
  remote: [] as Entry[],
  // Boxes from the server that this account's key does not open, or that failed a check.
  unreadable: 0,
  loaded: false,
  // A key action running right now ('' when idle), its last error and its last good news.
  busy: '',
  err: '',
  note: '',
  // The last automatic backup that failed, so a backup that didn't happen is never silent.
  autoErr: '',
  // The username whose session the server ended (sessions last 2 hours), so the screens can ask
  // to sign in again instead of acting as if backup were off. '' when signed in or signed out on purpose.
  ended: '',
});

export async function sync() {
  const s = await engine.backupStatus();
  if (cloud.signedIn && !s.signedIn) cloud.ended = cloud.username;
  // Signed in again: the failures from before are history.
  if (s.signedIn && !cloud.signedIn) cloud.ended = cloud.autoErr = cloud.err = '';
  cloud.signedIn = s.signedIn;
  cloud.username = s.username;
  if (!s.signedIn) {
    cloud.remote = [];
    cloud.unreadable = 0;
    cloud.loaded = false;
  }
}

export async function load() {
  const l = await engine.backupList();
  cloud.remote = l.entries;
  cloud.unreadable = l.unreadable;
  cloud.loaded = true;
}

/** Signed out by the app (vault locked or deleted): forget what was shown. */
export function reset() {
  Object.assign(cloud, { signedIn: false, username: '', remote: [], unreadable: 0, loaded: false, busy: '', err: '', note: '', autoErr: '', ended: '' });
}

const keys = (n: number) => `${n.toLocaleString('en-US')} ${n === 1 ? 'key' : 'keys'}`;

/** Runs one key action at a time. On a failure, even a "Don't send" halfway through a big job,
 *  the list is fetched again, since the batches before it are stored. */
async function run(what: string, fn: () => Promise<string>) {
  if (cloud.busy) return;
  cloud.busy = what;
  cloud.err = cloud.note = '';
  try {
    cloud.note = await fn();
  } catch (e) {
    cloud.err = String(e);
    await sync().catch(() => {});
    if (cloud.signedIn) await load().catch(() => {});
  }
  cloud.busy = '';
}

export const upload = (addresses: string[]) =>
  run('upload', async () => {
    const n = await engine.backupUpload(addresses);
    await load();
    return `${keys(n)} locked and stored in VanityCloud.`;
  });

export const restore = (addresses: string[], onChanged: () => void) =>
  run('restore', async () => {
    const n = await engine.backupRestore(addresses);
    onChanged();
    return `${keys(n)} restored to the vault on this PC.`;
  });

export const drop = (addresses: string[]) =>
  run('remove', async () => {
    await engine.backupRemove(addresses);
    await load();
    return 'Removed from VanityCloud.';
  });
