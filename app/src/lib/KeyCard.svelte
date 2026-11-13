<script lang="ts">
  // A recovery key, shown once, with its own label so the vault's and VanityCloud's are never
  // mixed up: violet for the vault, cyan for VanityCloud. Print gives a sheet with only this key.
  import { engine } from './engine';

  let { kind, value, username = '' }: { kind: 'vault' | 'cloud'; value: string; username?: string } = $props();

  const NAME = { vault: 'Vault recovery key', cloud: 'VanityCloud recovery key' };
  const OPENS = {
    vault: 'Opens the vault on this PC if you forget your vault password.',
    cloud: 'Opens your VanityCloud backup, on any PC, if you forget your VanityCloud password.',
  };
  const today = new Date().toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });
  let copied = $state(false);

  async function copy() {
    await engine.copyPrivate(value);
    copied = true;
    setTimeout(() => (copied = false), 4000);
  }
</script>

<div class="kc {kind}">
  <span class="kt {kind}">{NAME[kind]}</span>
  <p class="what">{OPENS[kind]}</p>
  <div class="rkey mono">{value}</div>
  <div class="acts">
    <button class="link" onclick={copy}>{copied ? 'Copied. Clears from the clipboard in 30 s' : 'Copy'}</button>
    <button class="link" onclick={() => window.print()}>Print</button>
  </div>
</div>

<!-- The printed sheet: only this, nothing else from the window. -->
<div class="paper">
  <h1>VanityVault &middot; {NAME[kind]}</h1>
  <p>{OPENS[kind]}</p>
  {#if kind === 'cloud' && username}<p>VanityCloud username: <b>{username}</b></p>{/if}
  <p class="k">{value}</p>
  <p>Printed {today}. Keep this sheet somewhere safe and private. It is the only copy: VanityVault doesn't store it.</p>
  <p>Not the same as your {kind === 'vault' ? 'VanityCloud recovery key' : 'vault recovery key'}.</p>
</div>

<style>
  .kc {
    width: 100%;
    margin-top: 24px;
    padding: 16px 18px 12px;
    border: 1px solid var(--line2);
    border-radius: 16px;
  }
  .kc.vault {
    border-color: rgb(153 69 255 / 0.4);
    background: rgb(153 69 255 / 0.05);
  }
  .kc.cloud {
    border-color: rgb(25 211 243 / 0.4);
    background: rgb(25 211 243 / 0.04);
  }
  .what {
    margin: 10px 0 0;
    color: var(--mute);
    font-size: 14px;
  }
  .kc :global(.rkey) {
    margin-top: 14px;
  }
  .acts {
    display: flex;
    gap: 20px;
  }
  .paper {
    display: none;
  }
  @media print {
    :global(body *) {
      visibility: hidden;
    }
    .paper,
    .paper * {
      visibility: visible;
    }
    .paper {
      display: block;
      position: fixed;
      inset: 0;
      padding: 48px;
      background: #fff;
      color: #000;
      font: 14px/1.5 'Segoe UI', system-ui, sans-serif;
    }
    .paper h1 {
      font-size: 22px;
      margin: 0 0 12px;
    }
    .paper .k {
      margin: 22px 0;
      padding: 18px;
      border: 2px solid #000;
      border-radius: 10px;
      font: 600 20px/1.4 Consolas, monospace;
      letter-spacing: 0.06em;
      text-align: center;
    }
  }
</style>
