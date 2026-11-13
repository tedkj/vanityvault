<script lang="ts">
  // "Delete everything": the VanityCloud account (if signed in), the vault on this PC and what the
  // window remembers (hunt list, settings). Order matters: the vault password is checked first,
  // then the online account goes (through the Send window), and only then the vault. If the online
  // step fails or is cancelled, nothing on this PC has been touched.
  import { engine, type BackupStatus } from './engine';

  let { count, onClose, onDone }: { count: number; onClose: () => void; onDone: () => void } = $props();

  let cloud = $state<BackupStatus | null>(null);
  let vaultPw = $state('');
  let cloudPw = $state('');
  let typed = $state('');
  let busy = $state('');
  let err = $state('');

  engine.backupStatus().then((s) => (cloud = s));

  const keys = (n: number) => `${n} ${n === 1 ? 'key' : 'keys'}`;
  const ready = $derived(!!cloud && !!vaultPw && (!cloud.signedIn || !!cloudPw) && typed.trim() === 'DELETE' && !busy);

  async function wipe() {
    if (!ready || !cloud) return;
    err = '';
    try {
      busy = 'Checking your vault password';
      await engine.vaultCheck(vaultPw);
      if (cloud.signedIn) {
        busy = 'Deleting your VanityCloud account';
        await engine.backupDeleteAccount(cloudPw);
      }
      busy = 'Deleting the vault on this PC';
      await engine.vaultDestroy(vaultPw);
      try {
        localStorage.clear();
      } catch {
        // nothing remembered, nothing to clear
      }
      onDone();
    } catch (e) {
      err = String(e);
      busy = '';
    }
  }
  const onKey = (e: KeyboardEvent) => e.key === 'Escape' && !busy && onClose();
</script>

<svelte:window onkeydown={onKey} />

<div class="veil">
  <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="wipe-t">
    <div class="label bad">Delete everything</div>
    <h2 id="wipe-t">Delete everything and start over?</h2>
    <p>This deletes, for good:</p>
    <ul>
      <li>The vault on this PC, with {keys(count)} in it</li>
      {#if cloud?.signedIn}<li>Your VanityCloud account <b>{cloud.username}</b> and every locked box in it</li>{/if}
      <li>Your hunt list and settings on this PC</li>
    </ul>
    {#if cloud && !cloud.signedIn}
      <p class="note">Have a VanityCloud account? Sign in under Backup first to delete it too. Otherwise it stays online.</p>
    {/if}
    <div class="box">Keys you haven't copied somewhere else are gone for good. This can't be undone.</div>

    <span class="kt vault fld">Vault password</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input class="in" type="password" bind:value={vaultPw} placeholder="Vault password" autofocus />
    {#if cloud?.signedIn}
      <span class="kt cloud fld">VanityCloud password</span>
      <input class="in" type="password" bind:value={cloudPw} placeholder="VanityCloud password" />
    {/if}
    <input class="in" bind:value={typed} placeholder="Type DELETE to confirm" spellcheck="false" autocomplete="off" />

    {#if err}<div class="hint bad">{err}</div>{/if}
    <div class="row end">
      <button class="btn" disabled={!!busy} onclick={onClose}>Keep everything</button>
      <button class="btn danger" disabled={!ready} onclick={wipe}>
        {#if busy}{busy}<span class="dots"></span>{:else}Delete everything{/if}
      </button>
    </div>
  </div>
</div>

<style>
  ul {
    margin: 0 0 14px;
    padding-left: 20px;
    color: var(--mute);
    font-size: 14.5px;
  }
  li b {
    color: var(--text);
    font-weight: 500;
  }
  .note {
    font-size: 13.5px;
  }
  .box {
    margin-bottom: 4px;
    padding: 12px 14px;
    border: 1px dashed rgba(255, 93, 93, 0.45);
    border-radius: 12px;
    font-size: 13.5px;
    color: var(--mute);
  }
  .danger {
    border-color: rgba(255, 93, 93, 0.6);
    color: var(--red);
  }
</style>
