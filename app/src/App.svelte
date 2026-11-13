<script lang="ts">
  import Hunt from './screens/Hunt.svelte';
  import Vault from './screens/Vault.svelte';
  import Backup from './screens/Backup.svelte';
  import AboutScreen from './screens/About.svelte';
  import Faq from './screens/Faq.svelte';
  import Gate from './screens/Gate.svelte';
  import NetAsk from './lib/NetAsk.svelte';
  import { engine, win, type About, type Entry, type Status } from './lib/engine';
  import { prefs } from './lib/prefs';

  type Tab = 'hunt' | 'vault' | 'backup' | 'faq' | 'about';
  let tab = $state<Tab>('hunt');
  let status = $state<Status | null>(null);
  let entries = $state<Entry[]>([]);
  let backupOn = $state(false);
  // Automatic backup of new finds: bumped after each one so the Backup screen reloads, and the
  // last failure, so a backup that didn't happen is never silent.
  let synced = $state(0);
  let autoErr = $state('');
  let about = $state<About>({ threads: 0, card: { state: 'starting', name: '' } });

  const refresh = () => engine.list().then((e) => (entries = e));

  // New finds go to VanityCloud by themselves when the user chose that and is signed in. Bursts
  // of hunt list finds are gathered into one upload (one Send window).
  let autoTimer = 0;
  function autoBackup() {
    if (!backupOn || !prefs.autoBackup) return;
    clearTimeout(autoTimer);
    autoTimer = window.setTimeout(async () => {
      try {
        if (await engine.backupUploadNew()) synced++;
        autoErr = '';
      } catch (e) {
        autoErr = `New finds were not backed up: ${e}`;
      }
    }, 1500);
  }
  const found = () => refresh().then(autoBackup);

  $effect(() => {
    engine.about().then((a) => (about = a));
    engine.status().then((s) => {
      status = s;
      if (s === 'open') refresh();
    });
    const offAbout = engine.onAbout((a) => (about = a));
    const offHit = engine.onHit(found);
    // Hunt list finds can arrive in bursts, so the vault list is reloaded once they settle.
    let soon = 0;
    const offFind = engine.onFind(() => (clearTimeout(soon), (soon = window.setTimeout(found, 300))));
    return () => (offAbout(), offHit(), offFind(), clearTimeout(soon), clearTimeout(autoTimer));
  });

  function opened() {
    status = 'open';
    engine.backupStatus().then((s) => (backupOn = s.signedIn));
    tab = 'hunt';
    refresh();
  }
  // "Delete everything" finished: the vault is gone, so back to making a new one.
  function deleted() {
    entries = [];
    backupOn = false;
    autoErr = '';
    tab = 'hunt';
    status = 'none';
  }
  async function lock() {
    await engine.lock();
    entries = [];
    backupOn = false;
    status = 'locked';
  }
</script>

<div class="shell">
  <header data-tauri-drag-region>
    <div class="logo" data-tauri-drag-region>
      <svg viewBox="0 0 24 24" fill="none">
        <defs>
          <linearGradient id="lg" x1="0" x2="24" y1="0" y2="0" gradientUnits="userSpaceOnUse">
            <stop stop-color="#9945ff" /><stop offset=".5" stop-color="#19d3f3" /><stop offset="1" stop-color="#14f195" />
          </linearGradient>
        </defs>
        <path d="M2 5l5.5 14L13 5M11 5l5.5 14L22 5" stroke="url(#lg)" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      VanityVault
    </div>
    <div class="wc">
      <button aria-label="Minimize" onclick={win.minimize}><svg viewBox="0 0 10 10"><path d="M1 5h8" /></svg></button>
      <button aria-label="Maximize" onclick={win.toggleMaximize}><svg viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" /></svg></button>
      <button aria-label="Close" class="x" onclick={win.close}><svg viewBox="0 0 10 10"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg></button>
    </div>
  </header>

  {#if status === 'open'}
    <nav>
      <button class:on={tab === 'hunt'} onclick={() => (tab = 'hunt')}>Hunt</button>
      <button class:on={tab === 'vault'} onclick={() => (tab = 'vault')}>Vault <span>{entries.length || ''}</span></button>
      <button class:on={tab === 'backup'} onclick={() => (tab = 'backup')}>Backup <span>{backupOn ? 'on' : 'off'}</span></button>
      <button class:on={tab === 'faq'} onclick={() => (tab = 'faq')}>FAQ</button>
      <button class:on={tab === 'about'} onclick={() => (tab = 'about')}>About</button>
      <div class="eng">
        <div class="label">Engine</div>
        {#if about.card.state === 'ready'}
          <b>GPU</b>
          <div>{about.card.name}</div>
        {:else}
          <b>CPU</b>
          {#if about.threads}<div>{about.threads} threads on this PC</div>{/if}
          {#if about.card.state === 'starting'}
            <div class="busy-pulse shaders">Compiling shaders</div>
            <div class="soon">
              One-time setup for your GPU, about 2&ndash;4 minutes. You can hunt right away: it starts on the CPU and moves to the GPU
              by itself when the shaders are done.
            </div>
          {:else}
            <div class="soon">No usable GPU found, so hunts run on the CPU.</div>
          {/if}
        {/if}
      </div>
      <button class="lock" onclick={lock}>Lock vault</button>
    </nav>

    <main>
      <div class="pane" class:show={tab === 'hunt'}><Hunt {about} onOpenVault={() => (tab = 'vault')} /></div>
      {#if tab === 'vault'}<div class="pane show"><Vault {entries} onHunt={() => (tab = 'hunt')} onChanged={refresh} onDeleted={deleted} /></div>{/if}
      {#if tab === 'backup'}<div class="pane show"><Backup {entries} {synced} {autoErr} onChanged={refresh} onState={(on) => (backupOn = on)} /></div>{/if}
      {#if tab === 'faq'}<div class="pane show"><Faq /></div>{/if}
      {#if tab === 'about'}<div class="pane show"><AboutScreen /></div>{/if}
    </main>
  {:else if status}
    <main class="full">{#key status}<Gate {status} onOpen={opened} />{/key}</main>
  {/if}
  <NetAsk />
</div>

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template: 44px 1fr / 204px 1fr;
    border: 1px solid var(--line);
  }
  header {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--line);
    padding-left: 16px;
  }
  .logo {
    display: flex;
    align-items: center;
    gap: 9px;
    font-weight: 600;
    font-size: 13.5px;
    letter-spacing: -0.02em;
  }
  .logo svg {
    width: 17px;
    height: 17px;
    pointer-events: none;
  }
  .wc {
    display: flex;
    height: 100%;
  }
  .wc button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    transition: background 0.15s;
  }
  .wc svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: var(--mute);
    stroke-width: 1;
  }
  .wc button:hover {
    background: rgba(255, 255, 255, 0.07);
  }
  .wc button.x:hover {
    background: #c42b1c;
  }
  .wc button:hover svg {
    stroke: var(--text);
  }
  nav {
    border-right: 1px solid var(--line);
    padding: 14px 10px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  nav button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 9px 12px;
    border-radius: 10px;
    color: var(--mute);
    font-size: 14px;
    text-align: left;
    transition:
      background 0.2s,
      color 0.2s;
  }
  nav button:hover {
    color: var(--text);
  }
  nav button.on {
    background: rgba(255, 255, 255, 0.055);
    color: var(--text);
  }
  nav button span {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--dim);
  }
  .eng {
    margin-top: auto;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: 12px;
    font-size: 12px;
    color: var(--mute);
    line-height: 1.7;
  }
  .eng b {
    display: block;
    color: var(--text);
    font-weight: 500;
    font-size: 13px;
    margin-top: 2px;
  }
  .eng .soon {
    color: var(--dim);
    line-height: 1.5;
  }
  .eng .shaders {
    margin-top: 8px;
    font-family: var(--mono);
    font-size: 12.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  nav button.lock {
    margin-top: 8px;
    justify-content: center;
    border: 1px solid var(--line);
    font-size: 13px;
  }
  main {
    position: relative;
    min-width: 0;
    min-height: 0;
  }
  main.full {
    grid-column: 1 / -1;
  }
  .pane {
    position: absolute;
    inset: 0;
    display: none;
  }
  .pane.show {
    display: block;
  }
</style>
