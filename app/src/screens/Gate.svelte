<script lang="ts">
  import { engine, type Status, type Strength } from '../lib/engine';
  import KeyCard from '../lib/KeyCard.svelte';

  let { status, onOpen }: { status: Status; onOpen: () => void } = $props();

  // The gate is remounted whenever the status changes, so reading it once is right.
  // svelte-ignore state_referenced_locally
  let mode = $state<'password' | 'recovery' | 'unlock' | 'forgot'>(status === 'none' ? 'password' : 'unlock');
  let password = $state('');
  let confirm = $state('');
  let recoveryIn = $state('');
  let recoveryOut = $state('');
  let saved = $state(false);
  let busy = $state(false);
  let err = $state('');
  let strength = $state<Strength>({ score: 0, ok: false, hint: '' });

  const setting = $derived(mode === 'password' || mode === 'forgot');
  const ready = $derived(
    mode === 'unlock' ? !!password : setting ? strength.ok && password === confirm && (mode === 'password' || !!recoveryIn.trim()) : saved,
  );
  const words = ['Too easy to guess', 'Too easy to guess', 'Still weak', 'Good', 'Strong'];

  $effect(() => {
    const p = password;
    if (!setting) return;
    let live = true;
    engine.passwordCheck(p).then((s) => live && (strength = s));
    return () => (live = false);
  });

  async function go() {
    if (!ready || busy) return;
    if (mode === 'recovery') return onOpen();
    busy = true;
    err = '';
    try {
      if (mode === 'password') {
        recoveryOut = await engine.create(password);
        mode = 'recovery';
      } else {
        await (mode === 'unlock' ? engine.unlock(password) : engine.recover(recoveryIn, password));
        onOpen();
      }
      password = confirm = '';
    } catch (e) {
      err = String(e);
    }
    busy = false;
  }
  function swap(m: typeof mode) {
    mode = m;
    password = confirm = err = '';
  }
  const enter = (e: KeyboardEvent) => e.key === 'Enter' && go();
</script>

<div class="gate">
  <div class="card">
    {#if mode === 'password'}
      <div class="label">Step 1 of 2</div>
      <h1>First, a password for your <em class="s">vault.</em></h1>
      <p class="lead">
        Every key you find is kept on this PC in one encrypted file, and your vault password locks it. Nobody can reset it for you, us included.
      </p>
    {:else if mode === 'recovery'}
      <div class="label">Step 2 of 2</div>
      <h1>Your vault recovery <em class="s grad">key.</em></h1>
      <p class="lead">
        If you ever forget your vault password, this is the only other way in. It's shown this one time only, so print it or write it down and keep
        it somewhere safe.
      </p>
      <KeyCard kind="vault" value={recoveryOut} />
      <label class="tick"><input type="checkbox" bind:checked={saved} /> I have saved my vault recovery key</label>
    {:else if mode === 'unlock'}
      <div class="label">Vault locked</div>
      <h1>Welcome <em class="s">back.</em></h1>
      <p class="lead">Type your vault password to open the vault.</p>
    {:else}
      <div class="label">Recovery</div>
      <h1>Open it with your recovery <em class="s">key.</em></h1>
      <p class="lead">Type the vault recovery key you saved when you made the vault, then choose a new vault password.</p>
      <span class="kt vault fld">Vault recovery key</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="in mono" bind:value={recoveryIn} placeholder="XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX" spellcheck="false" autofocus />
    {/if}

    {#if mode !== 'recovery'}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="in"
        type="password"
        bind:value={password}
        placeholder={mode === 'forgot' ? 'New vault password' : 'Vault password'}
        autofocus={mode !== 'forgot'}
        onkeydown={enter}
      />
      {#if setting}
        <div class="meter" data-score={strength.score}>{#each [1, 2, 3, 4] as n}<i class:lit={strength.score >= n}></i>{/each}</div>
        <div class="hint">
          {#if password}{words[strength.score]}. {strength.ok ? '' : strength.hint || 'A few unrelated words work well.'}{:else}A few unrelated
            words work well, for example four random words.{/if}
        </div>
        <input class="in" type="password" bind:value={confirm} placeholder="Type it again" onkeydown={enter} />
        {#if confirm && confirm !== password}<div class="hint bad">The two don't match yet.</div>{/if}
      {/if}
    {/if}

    {#if err}<div class="hint bad">{err}</div>{/if}

    <button class="btn pri" disabled={!ready || busy} onclick={go}>
      {#if busy}{mode === 'unlock' ? 'Unlocking' : 'Locking it in'}<span class="dots"></span>
      {:else if mode === 'password'}Create vault
      {:else if mode === 'recovery'}Continue
      {:else if mode === 'unlock'}Unlock
      {:else}Set new password{/if}
    </button>

    {#if mode === 'unlock'}<button class="link" onclick={() => swap('forgot')}>Forgot it? Use your vault recovery key</button>{/if}
    {#if mode === 'forgot'}<button class="link" onclick={() => swap('unlock')}>Back to password</button>{/if}
  </div>
</div>

<style>
  .gate {
    height: 100%;
    display: grid;
    place-items: center;
    overflow: auto;
    padding: 30px;
    background: radial-gradient(900px 500px at 50% 0%, rgba(153, 69, 255, 0.07), transparent 70%);
  }
  .card {
    width: min(560px, 100%);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    animation: enter 0.6s var(--ease);
  }
  h1 {
    margin-top: 14px;
  }
  .btn {
    margin-top: 24px;
    min-width: 190px;
  }
</style>
