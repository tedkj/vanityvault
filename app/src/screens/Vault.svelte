<script lang="ts">
  import { engine, type Entry } from '../lib/engine';
  import { big } from '../lib/format';
  import Addr from '../lib/Addr.svelte';
  import DeleteAll from '../lib/DeleteAll.svelte';

  let { entries, onHunt, onChanged, onDeleted }: { entries: Entry[]; onHunt: () => void; onChanged: () => void; onDeleted: () => void } =
    $props();
  let wiping = $state(false);

  // One key at a time can be open for an action.
  let open = $state<{ address: string; kind: 'secret' | 'saved' | 'delete' | 'error'; text: string } | null>(null);
  let copied = $state('');
  let hideTimer = 0;

  function flash(what: string, ms = 1400) {
    copied = what;
    setTimeout(() => (copied = ''), ms);
  }
  async function copy(text: string, what: string) {
    await navigator.clipboard.writeText(text);
    flash(what);
  }
  // A secret key goes through the core: kept out of clipboard history, cleared after 30 seconds.
  async function copySecret(text: string) {
    await engine.copyPrivate(text);
    flash('secret', 4000);
  }
  function close() {
    clearTimeout(hideTimer);
    open = null;
  }
  async function act(address: string, kind: 'secret' | 'saved', run: () => Promise<string>) {
    close();
    try {
      open = { address, kind, text: await run() };
      // A shown secret hides itself again after half a minute.
      if (kind === 'secret') hideTimer = window.setTimeout(close, 30000);
    } catch (e) {
      open = { address, kind: 'error', text: String(e) };
    }
  }
  async function remove(address: string) {
    try {
      await engine.remove(address);
      close();
      onChanged();
    } catch (e) {
      open = { address, kind: 'error', text: String(e) };
    }
  }
  const when = (ms: number) => new Date(ms).toLocaleString([], { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
</script>

<div class="screen">
  <div class="label">Vault</div>
  <h1>Your <em class="s">keys.</em></h1>

  {#if entries.length === 0}
    <p class="lead">Nothing here yet. Every address you find is locked in here automatically.</p>
    <button class="btn pri" style="margin-top:28px" onclick={onHunt}>Start a hunt</button>
  {:else}
    <p class="lead">Stored on this PC in one encrypted file. To use an address in a wallet, show its secret key or save its key file.</p>
    <div class="list">
      {#each entries as e (e.address)}
        {@const mine = open?.address === e.address ? open : null}
        <div class="item" class:active={!!mine}>
          <div class="a mono"><Addr address={e.address} prefixLen={e.prefixLen} suffixLen={e.suffixLen} /></div>
          <div class="m mono">
            <span>{when(e.at)} &middot; {big(e.tries)} tries</span>
            <span class="acts">
              <button onclick={() => copy(e.address, e.address)}>{copied === e.address ? 'copied' : 'copy address'}</button>
              <button onclick={() => act(e.address, 'secret', () => engine.secret(e.address))}>show secret key</button>
              <button onclick={() => act(e.address, 'saved', () => engine.exportFile(e.address))}>save key file</button>
              <button onclick={() => (close(), (open = { address: e.address, kind: 'delete', text: '' }))}>delete</button>
            </span>
          </div>

          {#if mine?.kind === 'secret'}
            <div class="box warn">
              <b>Anyone who sees this can take everything in this wallet.</b> In Phantom or Solflare, choose "Import private key" and paste it.
              <div class="secret mono">{mine.text}</div>
              <div class="row">
                <button class="btn pri sm" onclick={() => copySecret(mine.text)}>{copied === 'secret' ? 'Copied. Clears in 30 s' : 'Copy secret key'}</button>
                <button class="btn sm" onclick={close}>Hide</button>
              </div>
            </div>
          {:else if mine?.kind === 'saved'}
            <div class="box warn">
              <b>Saved.</b> That file is the full private key, with no password on it. Keep it somewhere safe and delete it when you no longer need
              it.
              <div class="secret mono">{mine.text}</div>
              <div class="row"><button class="btn sm" onclick={close}>OK</button></div>
            </div>
          {:else if mine?.kind === 'delete'}
            <div class="box warn">
              <b>Delete this key for good?</b> Anything held by this address is lost unless you have the key somewhere else. This can't be undone.
              <div class="row">
                <button class="btn sm danger" onclick={() => remove(e.address)}>Delete forever</button>
                <button class="btn sm" onclick={close}>Keep it</button>
              </div>
            </div>
          {:else if mine?.kind === 'error'}
            <div class="box warn"><b>That didn't work.</b> {mine.text}</div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <div class="over">
    <div class="label">Start over</div>
    <p>Delete the vault on this PC, your VanityCloud account and everything the app remembers here.</p>
    <button class="btn sm danger" onclick={() => (wiping = true)}>Delete everything</button>
  </div>
</div>

{#if wiping}<DeleteAll count={entries.length} onClose={() => (wiping = false)} onDone={onDeleted} />{/if}

<style>
  h1 {
    margin-top: 14px;
  }
  .list {
    margin-top: 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 920px;
  }
  .item {
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 15px 18px;
    background: var(--ink2);
    transition: border-color 0.2s;
  }
  .item:hover,
  .item.active {
    border-color: var(--line2);
  }
  .a {
    font-size: 15px;
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
    color: var(--dim);
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
  .box {
    margin-top: 14px;
    padding: 14px 16px;
    border: 1px dashed rgba(255, 93, 93, 0.4);
    border-radius: 12px;
    font-size: 13.5px;
    color: var(--mute);
    animation: enter 0.35s var(--ease);
  }
  .box b {
    color: var(--text);
    font-weight: 500;
  }
  .secret {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: 9px;
    background: var(--ink);
    color: var(--text);
    font-size: 12px;
    word-break: break-all;
    user-select: text;
  }
  .row {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  :global(.btn.sm) {
    height: 36px;
    padding: 0 16px;
    font-size: 13.5px;
  }
  .danger {
    border-color: rgba(255, 93, 93, 0.6);
    color: var(--red);
  }
  .over {
    margin-top: 44px;
    padding-top: 22px;
    border-top: 1px solid var(--line);
  }
  .over p {
    margin: 8px 0 14px;
    color: var(--mute);
    font-size: 14px;
  }
</style>
