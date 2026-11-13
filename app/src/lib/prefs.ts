// Small per-PC preferences, kept in the window's own storage. Nothing secret goes here.
const AUTO = 'vv.autoBackup';

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // storage unavailable: the setting just isn't remembered
  }
}

export const prefs = {
  /** Back up new finds to VanityCloud by themselves, while signed in. Off unless chosen. */
  get autoBackup() {
    return read(AUTO) === '1';
  },
  set autoBackup(on: boolean) {
    write(AUTO, on ? '1' : '0');
  },
};
