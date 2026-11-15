// Small per-PC preferences, kept in the window's own storage. Nothing secret goes here.
const AUTO = 'vv.autoBackup';
const SORT = 'vv.vaultSort';

export type VaultSort = 'newest' | 'longest' | 'az';

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
  /** The order the vault lists its keys in. Newest first unless chosen. */
  get vaultSort(): VaultSort {
    const v = read(SORT);
    return v === 'longest' || v === 'az' ? v : 'newest';
  },
  set vaultSort(order: VaultSort) {
    write(SORT, order);
  },
};
