<script lang="ts">
  import { engine, type Entry, type Strength } from '../lib/engine';
  import Addr from '../lib/Addr.svelte';
  import KeyCard from '../lib/KeyCard.svelte';
  import { prefs } from '../lib/prefs';

  let {
    entries,
    synced = 0,
    autoErr = '',
    onChanged,
    onState,
  }: { entries: Entry[]; synced?: number; autoErr?: string; onChanged: () => void; onState: (on: boolean) => void } = $props();
  let auto = $state(prefs.autoBackup);

  const SHOWN = 200;
  type Mode = 'intro' | 'create' | 'recovery' | 'signin' | 'forgot';

  // null until the core has said whether a session is open.
  let signedIn = $state<boolean | null>(null);
  let username = $state('');
  let mode = $state<Mode>('intro');
  let panel = $state<'' | 'password' | 'delete'>('');

  let name = $state('');
  let password = $state('');
  let confirm = $state('');
  let current = $state('');
  let recoveryIn = $state('');
  let recoveryOut = $state('');
  let agreed = $state(false);
  let saved = $state(false);
  let strength = $state<Strength>({ score: 0, ok: false, hint: '' });

  // What is running right now ('' when idle), the last error and the last good news.
  let busy = $state('');
  let err = $state('');
  let note = $state('');

  let remote = $state<Entry[]>([]);
  let unreadable = $state(0);
  let loaded = $state(false);
  // The address whose only copy is about to be removed from the backup.
  let asking = $state('');

  const setting = $derived(mode === 'create' || mode === 'forgot' || panel === 'password');
  const fresh = $derived(strength.ok && password === confirm);
  const here = $derived(new Set(entries.map((e) => e.address)));
  const there = $derived(new Set(remote.map((e) => e.address)));
  const toUpload = $derived(entries.filter((e) => !there.has(e.address)));
  const toRestore = $derived(remote.filter((e) => !here.has(e.address)));
  const rows = $derived([...entries, ...toRestore].sort((a, b) => b.at - a.at));
  const words = ['Too easy to guess', 'Too easy to guess', 'Still weak', 'Good', 'Strong'];
  const keys = (n: number) => `${n} ${n === 1 ? 'key' : 'keys'}`;

  $effect(() => {
    sync().then(() => {
      if (signedIn) load().catch((e) => (err = String(e)));
    });
  });
  // An automatic backup just ran: show the new state.
  $effect(() => {
    if (synced && signedIn) load().catch((e) => (err = String(e)));
  });
  $effect(() => {
    const p = password;
    if (!setting) return;
    let live = true;
    engine.passwordCheck(p).then((s) => live && (strength = s));
    return () => (live = false);
  });

  async function sync() {
    const s = await engine.backupStatus();
    signedIn = s.signedIn;
    username = s.username;
    onState(s.signedIn);
    if (!s.signedIn) {
      remote = [];
      loaded = false;
      panel = '';
    }
  }
  async function load() {
    const l = await engine.backupList();
    remote = l.entries;
    unreadable = l.unreadable;
    loaded = true;
  }
  function clear() {
    password = confirm = current = recoveryIn = err = note = '';
    agreed = false;
  }
  function go(m: Mode) {
    mode = m;
    clear();
  }
  function open(p: typeof panel) {
    panel = panel === p ? '' : p;
    clear();
  }
  /** Runs one action at a time. If it fails because the session ended, the screen falls back to sign-in. */
  async function run(what: string, fn: () => Promise<void>) {
    if (busy) return;
    busy = what;
    err = note = '';
    try {
      await fn();
    } catch (e) {
      err = String(e);
      await sync();
    }
    busy = '';
  }

  const create = () =>
    run('form', async () => {
      recoveryOut = await engine.backupRegister(name, password);
      password = confirm = '';
      saved = false;
      mode = 'recovery';
    });
  const enter = () =>
    run('form', async () => {
      await (mode === 'forgot' ? engine.backupRecover(name, recoveryIn, password) : engine.backupLogin(name, password));
      await entered();
    });
  async function entered() {
    recoveryOut = '';
    mode = 'intro';
    clear();
    await sync();
    await load();
  }
  const upload = (list: Entry[]) =>
    run('upload', async () => {
      const n = await engine.backupUpload(list.map((e) => e.address));
      await load();
      note = `${keys(n)} locked and stored in VanityCloud.`;
    });
  const restore = (list: Entry[]) =>
    run('restore', async () => {
      const n = await engine.backupRestore(list.map((e) => e.address));
      onChanged();
      note = `${keys(n)} restored to the vault on this PC.`;
    });
  const drop = (address: string) =>
    run('remove', async () => {
      asking = '';
      await engine.backupRemove([address]);
      await load();
      note = 'Removed from VanityCloud.';
    });
  const changePassword = () =>
    run('form', async () => {
      await engine.backupPassword(current, password);
      open('');
      note = 'Password changed. Other PCs signed in to this backup have been signed out.';
    });
  const deleteAccount = () =>
    run('form', async () => {
      await engine.backupDeleteAccount(current);
      clear();
      await sync();
      note = 'The backup account and everything in it has been deleted from the VanityCloud server.';
    });
  const signOut = () =>
    run('out', async () => {
      await engine.backupLogout();
      await sync();
    });
  const onEnter = (fn: () => void, ok: boolean) => (e: KeyboardEvent) => e.key === 'Enter' && ok && fn();
</script>

{#snippet newPassword(placeholder: string)}
  <input class="in" type="password" bind:value={password} {placeholder} />
  <div class="meter" data-score={strength.score}>{#each [1, 2, 3, 4] as n}<i class:lit={strength.score >= n}></i>{/each}</div>
  <div class="hint">
    {#if password}{words[strength.score]}. {strength.ok ? '' : strength.hint || 'A few unrelated words work well.'}{:else}A few unrelated words
      work well, for example four random words.{/if}
  </div>
  <input class="in" type="password" bind:value={confirm} placeholder="Type it again" />
  {#if confirm && confirm !== password}<div class="hint bad">The two don't match yet.</div>{/if}
{/snippet}

<div class="screen">
  {#if signedIn === null}
    <div class="label">Backup</div>
  {:else if mode === 'recovery'}
    <div class="form">
      <div class="label">Backup &middot; step 2 of 2</div>
      <h1>Your VanityCloud recovery <em class="s grad">key.</em></h1>
      <p class="lead">
        If you forget your VanityCloud password, this is the only other way in. It's not the same as your vault recovery key. It's shown this one
        time only, so print it or write it down.
      </p>
      <KeyCard kind="cloud" value={recoveryOut} username={name.trim().toLowerCase()} />
      <label class="tick"><input type="checkbox" bind:checked={saved} /> I have saved my VanityCloud recovery key</label>
      <button class="btn pri" disabled={!saved || !!busy} onclick={() => run('form', entered)}>Continue</button>
    </div>
  {:else if !signedIn && mode === 'intro'}
    <div class="label">Backup &middot; optional &middot; free</div>
    <h1>A backup <em class="s">we</em> can't open.</h1>
    <p class="lead">
      Keep a spare copy of your keys online, in case this PC dies. The app locks them with your VanityCloud password before they leave this PC, so the
      VanityCloud server only ever holds a locked box.
    </p>
    <div class="steps">
      <div>
        <!-- A PC with a password being typed. -->
        <svg viewBox="0 0 200 100" aria-hidden="true">
          <defs>
            <linearGradient id="bkg" x1="40" x2="160" y1="0" y2="0" gradientUnits="userSpaceOnUse">
              <stop stop-color="#9945ff" /><stop offset=".5" stop-color="#19d3f3" /><stop offset="1" stop-color="#14f195" />
            </linearGradient>
          </defs>
          <rect class="body" x="40" y="6" width="120" height="70" rx="9" />
          <path class="wire" d="M100 76V89M74 90H126" />
          <rect class="lit" x="54" y="30" width="92" height="24" rx="12" />
          {#each [0, 1, 2, 3, 4, 5] as i}<circle class="dot" cx={70 + i * 12} cy="42" r="3.2" style="--i:{i}" />{/each}
        </svg>
        <span class="label">On your PC</span><b>Your password</b>Never sent anywhere.
      </div>
      <div>
        <!-- A key travelling into a padlock. -->
        <svg viewBox="0 0 200 100" aria-hidden="true">
          <circle class="key" cx="40" cy="50" r="11" />
          <path class="key" d="M51 50H86M72 50V59M80 50V56" />
          <path class="wire" d="M94 50H128" />
          <path class="pulse" d="M94 50H128" />
          <path class="shackle" d="M140 46V35a12 12 0 0 1 24 0V46" />
          <rect class="lit glow" x="132" y="46" width="40" height="32" rx="7" />
          <path class="hole" d="M152 58v8" />
        </svg>
        <span class="label">On your PC</span><b>Keys get locked</b>With a key that never leaves.
      </div>
      <div>
        <!-- A cloud holding a locked box, with locked data arriving from below. -->
        <svg viewBox="0 0 200 100" aria-hidden="true">
          <path class="body" d="M58 80H146A20 20 0 0 0 150 41A30 30 0 0 0 94 28A24 24 0 0 0 54 42A19.5 19.5 0 0 0 58 80Z" />
          <path class="wire" d="M100 80V100" />
          <path class="pulse up" d="M100 100V80" />
          <path class="shackle" d="M92 50V45a8 8 0 0 1 16 0V50" />
          <rect class="lit" x="84" y="50" width="32" height="22" rx="5" />
          <path class="hole" d="M100 58v6" />
        </svg>
        <span class="label">On VanityCloud</span><b>A locked box</b>Unreadable to us.
      </div>
    </div>
    <div class="see">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12Z" /><circle cx="12" cy="12" r="3" /></svg>
      <p><b>You see everything we send.</b> Each time the app sends data to the VanityCloud server, a window first shows you the exact message, byte for byte. Nothing leaves this PC until you click Send. Only fetching your locked boxes and signing out skip it: they send no data, just your session pass.</p>
    </div>
    {#if note}<div class="hint good">{note}</div>{/if}
    {#if err}<div class="hint bad">{err}</div>{/if}
    <div class="row top">
      <button class="btn pri" onclick={() => go('create')}>Create a backup account</button>
      <button class="btn" onclick={() => go('signin')}>I already have one</button>
    </div>
  {:else if !signedIn}
    <div class="form">
      {#if mode === 'create'}
        <div class="label">Backup &middot; step 1 of 2</div>
        <h1>A name and a password for your <em class="s">backup.</em></h1>
        <p class="lead">Select a username: at least 6 characters, and not your email or real name. We don't want to know who you are. Your VanityCloud password locks your keys before they are sent, so it is never sent itself. It's separate from your vault password.</p>
      {:else if mode === 'signin'}
        <div class="label">Backup</div>
        <h1>Sign in to your <em class="s">backup.</em></h1>
        <p class="lead">Your keys are fetched as locked boxes and opened on this PC.</p>
      {:else}
        <div class="label">Backup &middot; recovery</div>
        <h1>Get back in with your recovery <em class="s">key.</em></h1>
        <p class="lead">Type the VanityCloud recovery key you saved when you made the account, then choose a new VanityCloud password.</p>
      {/if}

      <!-- svelte-ignore a11y_autofocus -->
      <input class="in" bind:value={name} placeholder="Username" spellcheck="false" autocomplete="off" maxlength="32" autofocus />
      {#if mode === 'forgot'}
        <span class="kt cloud fld">VanityCloud recovery key</span>
        <input class="in mono" bind:value={recoveryIn} placeholder="XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX" spellcheck="false" />
      {/if}
      {#if mode === 'signin'}
        <input class="in" type="password" bind:value={password} placeholder="VanityCloud password" onkeydown={onEnter(enter, !!name && !!password)} />
      {:else}
        {@render newPassword(mode === 'forgot' ? 'New VanityCloud password' : 'VanityCloud password')}
      {/if}
      {#if mode === 'create'}
        <label class="tick">
          <input type="checkbox" bind:checked={agreed} />
          <span>I understand that nobody can reset this password. If I lose it and the VanityCloud recovery key, the backup stays locked for good.</span>
        </label>
      {/if}

      {#if err}<div class="hint bad">{err}</div>{/if}

      {#if mode === 'create'}
        <button class="btn pri" disabled={!name.trim() || !fresh || !agreed || !!busy} onclick={create}>
          {#if busy}Creating<span class="dots"></span>{:else}Create account{/if}
        </button>
        <button class="link" onclick={() => go('signin')}>I already have an account</button>
      {:else if mode === 'signin'}
        <button class="btn pri" disabled={!name.trim() || !password || !!busy} onclick={enter}>
          {#if busy}Signing in<span class="dots"></span>{:else}Sign in{/if}
        </button>
        <button class="link" onclick={() => go('forgot')}>Forgot the password? Use your VanityCloud recovery key</button>
        <button class="link" onclick={() => go('create')}>Create an account instead</button>
      {:else}
        <button class="btn pri" disabled={!name.trim() || !recoveryIn.trim() || !fresh || !!busy} onclick={enter}>
          {#if busy}Setting it<span class="dots"></span>{:else}Set new password{/if}
        </button>
        <button class="link" onclick={() => go('signin')}>Back to sign in</button>
      {/if}
    </div>
  {:else}
    <div class="label">Backup &middot; signed in as {username}</div>
    <h1>Your spare <em class="s">copy.</em></h1>
    <p class="lead">Locked on this PC, stored online. Back up what you want to keep safe; restore it on any PC by signing in.</p>

    <div class="tiles">
      <div><span class="label">On this PC</span><b>{entries.length}</b></div>
      <div><span class="label">In VanityCloud</span><b>{loaded ? remote.length : '...'}</b></div>
      <div class:warn={loaded && toUpload.length > 0}><span class="label">This PC only</span><b>{loaded ? toUpload.length : '...'}</b></div>
    </div>

    <div class="row top">
      {#if loaded && toUpload.length}
        <button class="btn pri" disabled={!!busy} onclick={() => upload(toUpload)}>
          {#if busy === 'upload'}Backing up<span class="dots"></span>{:else}Back up {keys(toUpload.length)}{/if}
        </button>
      {/if}
      {#if loaded && toRestore.length}
        <button class="btn" disabled={!!busy} onclick={() => restore(toRestore)}>
          {#if busy === 'restore'}Restoring<span class="dots"></span>{:else}Restore {keys(toRestore.length)} to this PC{/if}
        </button>
      {/if}
      {#if loaded && !toUpload.length && !toRestore.length}
        <span class="calm mono">{entries.length ? '✓ everything on this PC is also in VanityCloud' : 'nothing to back up yet'}</span>
      {/if}
    </div>
    {#if note}<div class="hint good">{note}</div>{/if}
    {#if err}<div class="hint bad">{err}</div>{/if}
    {#if autoErr}<div class="hint bad">{autoErr}</div>{/if}
    <label class="tick auto">
      <input type="checkbox" checked={auto} onchange={(e) => (prefs.autoBackup = auto = e.currentTarget.checked)} />
      <span>Back up new finds automatically <em>Each upload still shows the Send window first, unless you've chosen to stop showing it.</em></span>
    </label>
    {#if unreadable}
      <div class="box">
        <b>{unreadable} {unreadable === 1 ? 'box' : 'boxes'} in the backup could not be opened.</b> They were not made by this account's key or were changed
        on the VanityCloud server, so the app ignores them.
      </div>
    {/if}

    {#if loaded && rows.length}
      <div class="list">
        {#each rows.slice(0, SHOWN) as e (e.address)}
          {@const onPc = here.has(e.address)}
          {@const inBackup = there.has(e.address)}
          <div class="item">
            <div class="a mono"><Addr address={e.address} prefixLen={e.prefixLen} suffixLen={e.suffixLen} /></div>
            <div class="m mono">
              <span class="where">
                <span class="on-label">Encrypted backup on:</span>
                {#if onPc}<i>{inBackup ? 'this PC' : 'this PC only'}</i>{/if}
                {#if inBackup}<i>{onPc ? 'VanityCloud' : 'VanityCloud only'}</i>{/if}
              </span>
              <span class="acts">
                {#if !inBackup}<button disabled={!!busy} onclick={() => upload([e])}>back up</button>{/if}
                {#if !onPc}<button disabled={!!busy} onclick={() => restore([e])}>restore to this PC</button>{/if}
                {#if inBackup}
                  <button disabled={!!busy} onclick={() => (onPc ? drop(e.address) : (asking = e.address))}>remove from VanityCloud</button>
                {/if}
              </span>
            </div>
            {#if asking === e.address}
              <div class="box">
                <b>This key is only in VanityCloud.</b> If you remove it, it is gone for good, and anything held by this address is lost.
                <div class="row">
                  <button class="btn sm danger" onclick={() => drop(e.address)}>Remove forever</button>
                  <button class="btn sm" onclick={() => (asking = '')}>Keep it</button>
                </div>
              </div>
            {/if}
          </div>
        {/each}
        {#if rows.length > SHOWN}<div class="more mono">and {rows.length - SHOWN} more, newest shown first</div>{/if}
      </div>
    {/if}

    <div class="account">
      <div class="label">Account</div>
      <div class="row">
        <button class="btn sm" onclick={() => open('password')}>Change password</button>
        <button class="btn sm" disabled={!!busy} onclick={signOut}>Sign out</button>
        <button class="btn sm danger" onclick={() => open('delete')}>Delete account</button>
      </div>
      {#if panel === 'password'}
        <div class="form inset">
          <input class="in" type="password" bind:value={current} placeholder="Current VanityCloud password" />
          {@render newPassword('New VanityCloud password')}
          <button class="btn pri" disabled={!current || !fresh || !!busy} onclick={changePassword}>
            {#if busy === 'form'}Changing<span class="dots"></span>{:else}Change password{/if}
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if panel === 'delete'}
  {@const cloudOnly = toRestore.length}
  <div class="veil">
    <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="del-t">
      <div class="label bad">Delete backup account</div>
      <h2 id="del-t">Delete your VanityCloud backup?</h2>
      <p>
        This deletes the account <b>{username}</b> and every locked box in it from the VanityCloud server. It can't be undone. The keys in the vault on
        this PC stay where they are.
      </p>
      {#if cloudOnly}
        <div class="box">
          <b>{keys(cloudOnly)} {cloudOnly === 1 ? 'is' : 'are'} only in VanityCloud</b> and would be gone for good, with anything held by
          {cloudOnly === 1 ? 'that address' : 'those addresses'}. Restore {cloudOnly === 1 ? 'it' : 'them'} to this PC first.
        </div>
      {/if}
      <!-- svelte-ignore a11y_autofocus -->
      <input class="in" type="password" bind:value={current} placeholder="VanityCloud password" autofocus onkeydown={onEnter(deleteAccount, !!current && !busy)} />
      {#if err}<div class="hint bad">{err}</div>{/if}
      <div class="row end">
        <button class="btn" onclick={() => open('')}>Keep it</button>
        <button class="btn danger" disabled={!current || !!busy} onclick={deleteAccount}>
          {#if busy === 'form'}Deleting<span class="dots"></span>{:else}Delete for good{/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<svelte:window onkeydown={(e) => e.key === 'Escape' && panel === 'delete' && !busy && open('')} />

<style>
  .see {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    max-width: 860px;
    margin-top: 14px;
    padding: 14px 18px;
    border: 1px solid rgb(20 241 149 / 0.25);
    border-radius: 14px;
    background: rgb(20 241 149 / 0.04);
  }
  .see svg {
    flex: none;
    width: 20px;
    height: 20px;
    margin-top: 1px;
    fill: none;
    stroke: var(--green);
    stroke-width: 1.8;
  }
  .see p {
    margin: 0;
    color: var(--mute);
    font-size: 14px;
  }
  .see b {
    color: var(--text);
    font-weight: 500;
  }
  h1 {
    margin-top: 14px;
  }
  .steps,
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-top: 30px;
    max-width: 860px;
  }
  .steps div,
  .tiles div {
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 16px 18px;
    background: var(--ink2);
    color: var(--mute);
    font-size: 14px;
  }
  /* The drawings in the three cards, in the same line style as the CPU and GPU on the hunt screen. */
  .steps svg {
    display: block;
    height: 96px;
    margin: 4px auto 16px;
    overflow: visible;
  }
  .steps .body {
    fill: var(--ink3);
    stroke: rgba(255, 255, 255, 0.16);
  }
  .steps .wire {
    fill: none;
    stroke: rgba(255, 255, 255, 0.16);
    stroke-linecap: round;
  }
  .steps .lit {
    fill: #07090c;
    stroke: url(#bkg);
    stroke-width: 1.2;
  }
  .steps .glow {
    animation: lockglow 2.2s ease-in-out infinite;
  }
  .steps .key,
  .steps .shackle {
    fill: none;
    stroke: rgba(255, 255, 255, 0.4);
    stroke-width: 2.4;
    stroke-linecap: round;
  }
  .steps .hole {
    stroke: var(--green);
    stroke-width: 3;
    stroke-linecap: round;
  }
  /* The password dots appear one by one, as if typed. */
  .steps .dot {
    fill: var(--green);
    opacity: 0.12;
    animation: typed 3.4s infinite;
    animation-delay: calc(var(--i) * 0.24s);
  }
  .steps .pulse {
    fill: none;
    stroke: var(--green);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-dasharray: 7 60;
    stroke-dashoffset: 7;
    animation: travel 1.3s linear infinite;
  }
  .steps .pulse.up {
    stroke: var(--violet);
    animation-duration: 1.1s;
  }
  @keyframes typed {
    10%,
    78% {
      opacity: 1;
    }
    92%,
    100% {
      opacity: 0.12;
    }
  }
  @keyframes travel {
    to {
      stroke-dashoffset: -60;
    }
  }
  @keyframes lockglow {
    50% {
      filter: drop-shadow(0 0 7px rgba(20, 241, 149, 0.7));
    }
  }
  .steps b {
    display: block;
    color: var(--text);
    font-weight: 500;
    font-size: 16px;
    margin: 8px 0 2px;
  }
  .tiles b {
    display: block;
    color: var(--text);
    font-weight: 500;
    font-size: 30px;
    letter-spacing: -0.04em;
    margin-top: 6px;
    font-variant-numeric: tabular-nums;
  }
  .tiles .warn {
    border-color: rgba(245, 184, 61, 0.45);
  }
  .form {
    width: min(560px, 100%);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .form .btn {
    margin-top: 24px;
    min-width: 190px;
  }
  .form.inset {
    margin-top: 6px;
  }
  .tick {
    align-items: flex-start;
    color: var(--mute);
  }
  .tick input {
    flex: none;
    margin-top: 2px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
  }
  .row.top {
    margin-top: 26px;
    min-height: 46px;
  }
  .calm {
    font-size: 12.5px;
    color: var(--green);
  }
  .hint.good {
    color: var(--green);
  }
  .tick.auto {
    align-items: flex-start;
    margin-top: 20px;
  }
  .tick.auto input {
    margin-top: 2px;
  }
  .tick.auto em {
    display: block;
    font-style: normal;
    font-size: 13px;
    color: var(--mute);
  }
  .list {
    margin-top: 26px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 920px;
  }
  .item {
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 13px 18px;
    background: var(--ink2);
  }
  .a {
    font-size: 14px;
    color: var(--mute);
    user-select: text;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .m {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    align-items: center;
    margin-top: 8px;
    font-size: 11px;
  }
  .where {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .on-label {
    color: var(--mute);
    margin-right: 2px;
  }
  .where i {
    font-style: normal;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid rgba(20, 241, 149, 0.35);
    color: var(--green);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .m button {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--mute);
    padding: 3px 10px;
    border: 1px solid var(--line2);
    border-radius: 999px;
    transition:
      color 0.2s,
      border-color 0.2s;
  }
  .m button:hover {
    color: var(--text);
    border-color: rgba(255, 255, 255, 0.34);
  }
  .m button:disabled {
    opacity: 0.4;
  }
  .more {
    font-size: 11px;
    color: var(--dim);
    padding: 6px 4px;
  }
  .box {
    margin-top: 14px;
    max-width: 920px;
    padding: 14px 16px;
    border: 1px dashed rgba(255, 93, 93, 0.4);
    border-radius: 12px;
    font-size: 13.5px;
    color: var(--mute);
  }
  .box b {
    color: var(--text);
    font-weight: 500;
  }
  .danger {
    border-color: rgba(255, 93, 93, 0.6);
    color: var(--red);
  }
  .account {
    margin-top: 40px;
    padding-top: 22px;
    border-top: 1px solid var(--line);
    max-width: 920px;
  }
</style>
