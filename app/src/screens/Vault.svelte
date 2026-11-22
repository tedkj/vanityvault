<script lang="ts">
  import { engine, type Entry } from '../lib/engine';
  import { big } from '../lib/format';
  import Addr from '../lib/Addr.svelte';
  import DeleteAll from '../lib/DeleteAll.svelte';
  import { prefs, type VaultSort } from '../lib/prefs';
  import { SvelteSet } from 'svelte/reactivity';
  import { cloud, PER_SEND, MAX_CLOUD, upload, restore, drop } from '../lib/cloud.svelte';

  let {
    entries,
    onHunt,
    onBackup,
    onChanged,
    onDeleted,
  }: { entries: Entry[]; onHunt: () => void; onBackup: () => void; onChanged: () => void; onDeleted: () => void } = $props();
  let wiping = $state(false);

  // Where each key is kept. Keys only in VanityCloud are listed too, so this is the one list of
  // every key; while signed out, the list is the vault on this PC alone.
  const on = $derived(!!cloud.signedIn && cloud.loaded);
  const here = $derived(new Set(entries.map((e) => e.address)));
  const there = $derived(new Set(on ? cloud.remote.map((e) => e.address) : []));
  const toUpload = $derived(on ? entries.filter((e) => !there.has(e.address)) : []);
  const toRestore = $derived(on ? cloud.remote.filter((e) => !here.has(e.address)) : []);
  const all = $derived([...entries, ...toRestore]);
  // VanityCloud holds MAX_CLOUD keys per account. The app never picks which keys go: when they
  // don't all fit, the user picks.
  const room = $derived(Math.max(0, MAX_CLOUD - cloud.remote.length));
  const keys = (n: number) => `${n.toLocaleString('en-US')} ${n === 1 ? 'key' : 'keys'}`;

  // Picked keys, to do one thing to many at once. Narrow the list down with the filters, pick
  // everything shown, then act.
  const picked = new SvelteSet<string>();
  const pick = (address: string, yes: boolean) => (yes ? picked.add(address) : picked.delete(address));
  const pickedRows = $derived(all.filter((e) => picked.has(e.address)));
  const pUpload = $derived(on ? pickedRows.filter((e) => here.has(e.address) && !there.has(e.address)) : []);
  const pRestore = $derived(on ? pickedRows.filter((e) => !here.has(e.address)) : []);
  const pCloud = $derived(on ? pickedRows.filter((e) => there.has(e.address)) : []);
  const pLocal = $derived(pickedRows.filter((e) => here.has(e.address)));
  // Asking before a bulk delete or removal: '' when not asking.
  let bulkAsk = $state<'' | 'delete' | 'drop'>('');
  const addrs = (list: Entry[]) => list.map((e) => e.address);
  function unpick(list: Entry[]) {
    for (const e of list) picked.delete(e.address);
    bulkAsk = '';
  }
  /** A cloud action on picked keys. The list is taken when clicked, and unpicked only if it worked. */
  async function bulk(list: Entry[], fn: (addresses: string[]) => Promise<void>) {
    await fn(addrs(list));
    if (!cloud.err) unpick(list);
  }
  let bulkErr = $state('');
  async function deleteLocal(list: Entry[]) {
    bulkErr = '';
    try {
      await engine.remove(addrs(list));
      unpick(list);
      onChanged();
    } catch (e) {
      bulkErr = String(e);
    }
  }

  // The order of the list. The word of a key is the part that was hunted for: the longer of its
  // start and its end.
  let sort = $state<VaultSort>(prefs.vaultSort);
  // Typing here keeps only the keys whose address has those characters in it, in either case.
  let look = $state('');
  // Which keys to show by where they are kept (only while signed in), and the longest word to show.
  let place = $state<'all' | 'pc' | 'both' | 'cloud'>('all');
  let upTo = $state(0);
  const filtered = $derived(!!look.trim() || place !== 'all' || upTo > 0);
  function order(by: VaultSort) {
    sort = by;
    prefs.vaultSort = by;
  }
  const word = (e: Entry) => (e.prefixLen >= e.suffixLen ? e.address.slice(0, e.prefixLen) : e.address.slice(e.address.length - e.suffixLen));
  const shown = $derived.by(() => {
    const newest = (a: Entry, b: Entry) => b.at - a.at;
    const az = (a: Entry, b: Entry) => word(a).toLowerCase().localeCompare(word(b).toLowerCase()) || newest(a, b);
    const longest = (a: Entry, b: Entry) => word(b).length - word(a).length || az(a, b);
    const want = look.trim().toLowerCase();
    const where = (e: Entry) =>
      !on || place === 'all'
        ? true
        : place === 'cloud'
          ? !here.has(e.address)
          : place === 'pc'
            ? here.has(e.address) && !there.has(e.address)
            : here.has(e.address) && there.has(e.address);
    const some = all.filter((e) => (!want || e.address.toLowerCase().includes(want)) && where(e) && (!upTo || word(e).length <= upTo));
    return some.sort(sort === 'longest' ? longest : sort === 'az' ? az : newest);
  });

  // One key at a time can be open for an action.
  // 'drop' asks before removing a key whose only copy is in VanityCloud.
  let open = $state<{ address: string; kind: 'secret' | 'saved' | 'delete' | 'drop' | 'error'; text: string } | null>(null);
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
      await engine.remove([address]);
      picked.delete(address);
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

  {#if all.length === 0}
    <p class="lead">Nothing here yet. Every address you find is locked in here automatically.</p>
    <button class="btn pri" style="margin-top:28px" onclick={onHunt}>Start a hunt</button>
  {:else}
    <p class="lead">Stored on this PC in one encrypted file. To use an address in a wallet, show its secret key or save its key file.</p>
    {#if on}
      <div class="cloud">
        <span class="label">VanityCloud</span>
        <span class="sum mono">{(all.length - toUpload.length).toLocaleString('en-US')} of {keys(all.length)} backed up</span>
        <span class="bulk">
          {#if toUpload.length && toUpload.length <= room}
            <button class="btn sm pri" disabled={!!cloud.busy} onclick={() => upload(addrs(toUpload))}>
              {#if cloud.busy === 'upload'}Backing up<span class="dots"></span>{:else}Back up {keys(toUpload.length)}{/if}
            </button>
          {/if}
          {#if toRestore.length}
            <button class="btn sm" disabled={!!cloud.busy} onclick={() => restore(toRestore.map((e) => e.address), onChanged)}>
              {#if cloud.busy === 'restore'}Restoring<span class="dots"></span>{:else}Restore {keys(toRestore.length)} to this PC{/if}
            </button>
          {/if}
          {#if !toUpload.length && !toRestore.length}<span class="calm mono">✓ every key is in both places</span>{/if}
        </span>
      </div>
      {#if toUpload.length > room}
        <div class="hint">
          VanityCloud holds up to {MAX_CLOUD.toLocaleString('en-US')} keys per account{room
            ? `, and ${keys(toUpload.length)} on this PC aren't in it yet, but only ${keys(room)} more fit. Pick the ones worth keeping: show "This PC only", tick them and back them up.`
            : ', and yours is full. To back up more, remove keys you don\'t need from VanityCloud first.'}
        </div>
      {:else if toUpload.length > PER_SEND}
        <div class="hint">
          That's {keys(toUpload.length)}, so they go up in {Math.ceil(toUpload.length / PER_SEND)} batches of up to {PER_SEND}. The Send window opens once for
          each batch, so click Send on each one. That's normal.
        </div>
      {/if}
      {#if cloud.note}<div class="hint good">{cloud.note}</div>{/if}
      {#if cloud.err}<div class="hint bad">{cloud.err}</div>{/if}
      {#if cloud.autoErr}<div class="hint bad">{cloud.autoErr}</div>{/if}
    {:else if cloud.ended && !cloud.signedIn}
      <div class="cloud">
        <span class="label">VanityCloud</span>
        <span class="sum mono">signed out after 12 hours without use</span>
        <span class="bulk"><button class="btn sm pri" onclick={onBackup}>Sign in again</button></span>
      </div>
    {/if}
    {#if all.length > 1}
      <div class="sort">
        <span class="label">Sort</span>
        <div class="seg">
          <button class:on={sort === 'newest'} onclick={() => order('newest')}>Newest</button>
          <button class:on={sort === 'longest'} onclick={() => order('longest')}>Longest word</button>
          <button class:on={sort === 'az'} onclick={() => order('az')}>Word A to Z</button>
        </div>
        <input class="look mono" bind:value={look} placeholder="Find in addresses" maxlength="44" spellcheck="false" autocomplete="off" />
      </div>
      <div class="sort">
        <span class="label">Show</span>
        {#if on}
          <div class="seg">
            <button class:on={place === 'all'} onclick={() => (place = 'all')}>All</button>
            <button class:on={place === 'pc'} onclick={() => (place = 'pc')}>This PC only</button>
            <button class:on={place === 'both'} onclick={() => (place = 'both')}>Backed up</button>
            <button class:on={place === 'cloud'} onclick={() => (place = 'cloud')}>VanityCloud only</button>
          </div>
        {/if}
        <select class="look len mono" bind:value={upTo}>
          <option value={0}>Words of any length</option>
          {#each [3, 4, 5, 6, 7, 8, 9] as n (n)}<option value={n}>Words up to {n} characters</option>{/each}
        </select>
        <span class="picks">
          {#if filtered}<span class="count mono">{shown.length} of {all.length}</span>{/if}
          <button class="pickall" disabled={!shown.length} onclick={() => shown.forEach((e) => picked.add(e.address))}
            >Pick {shown.length === all.length ? 'all' : 'all shown'}</button
          >
          {#if picked.size}<button class="pickall" onclick={() => (picked.clear(), (bulkAsk = ''))}>Pick none</button>{/if}
        </span>
      </div>
    {/if}
    {#if filtered && shown.length === 0}<p class="none">No key matches{look.trim() ? ` "${look.trim()}"` : ''} with these filters.</p>{/if}
    <div class="list">
      {#each shown as e (e.address)}
        {@const mine = open?.address === e.address ? open : null}
        {@const onPc = here.has(e.address)}
        {@const inCloud = there.has(e.address)}
        <div class="item" class:active={!!mine} class:picked={picked.has(e.address)}>
          <div class="a mono">
            <input
              type="checkbox"
              class="pick"
              aria-label="Pick this key"
              checked={picked.has(e.address)}
              onchange={(ev) => pick(e.address, ev.currentTarget.checked)}
            /><Addr address={e.address} prefixLen={e.prefixLen} suffixLen={e.suffixLen} />
          </div>
          <div class="m mono">
            <span class="info">
              {when(e.at)} &middot; {big(e.tries)} tries
              {#if on}
                {#if !onPc}<i class="tag far">VanityCloud only</i>{:else if inCloud}<i class="tag">backed up</i>{:else}<i class="tag dim">this PC only</i>{/if}
              {/if}
            </span>
            <span class="acts">
              <button onclick={() => copy(e.address, e.address)}>{copied === e.address ? 'copied' : 'copy address'}</button>
              {#if onPc}
                <button onclick={() => act(e.address, 'secret', () => engine.secret(e.address))}>show secret key</button>
                <button onclick={() => act(e.address, 'saved', () => engine.exportFile(e.address))}>save key file</button>
              {:else}
                <button disabled={!!cloud.busy} onclick={() => restore([e.address], onChanged)}>restore to this PC</button>
              {/if}
              {#if on && onPc && !inCloud && room}<button disabled={!!cloud.busy} onclick={() => upload([e.address])}>back up</button>{/if}
              {#if inCloud}
                <button disabled={!!cloud.busy} onclick={() => (onPc ? drop([e.address]) : (close(), (open = { address: e.address, kind: 'drop', text: '' })))}
                  >remove from VanityCloud</button
                >
              {/if}
              {#if onPc}<button onclick={() => (close(), (open = { address: e.address, kind: 'delete', text: '' }))}>delete</button>{/if}
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
              {#if inCloud}
                <b>Delete this key from this PC?</b> A copy stays in VanityCloud, so you can restore it later.
              {:else}
                <b>Delete this key for good?</b> Anything held by this address is lost unless you have the key somewhere else. This can't be undone.
              {/if}
              <div class="row">
                <button class="btn sm danger" onclick={() => remove(e.address)}>{inCloud ? 'Delete from this PC' : 'Delete forever'}</button>
                <button class="btn sm" onclick={close}>Keep it</button>
              </div>
            </div>
          {:else if mine?.kind === 'drop'}
            <div class="box warn">
              <b>This key is only in VanityCloud.</b> If you remove it, it is gone for good, and anything held by this address is lost.
              <div class="row">
                <button class="btn sm danger" onclick={() => (close(), drop([e.address]))}>Remove forever</button>
                <button class="btn sm" onclick={close}>Keep it</button>
              </div>
            </div>
          {:else if mine?.kind === 'error'}
            <div class="box warn"><b>That didn't work.</b> {mine.text}</div>
          {/if}
        </div>
      {/each}
    </div>

    {#if picked.size}
      {@const lostLocal = pLocal.filter((e) => !there.has(e.address)).length}
      {@const lostCloud = pCloud.filter((e) => !here.has(e.address)).length}
      <div class="bar">
        <div class="bar-row">
          <b class="mono">{picked.size.toLocaleString('en-US')} picked</b>
          <span class="bulk">
            {#if pUpload.length}
              <button
                class="btn sm pri"
                disabled={!!cloud.busy || pUpload.length > room}
                title={pUpload.length > room ? `Only ${keys(room)} more fit in VanityCloud` : ''}
                onclick={() => bulk(pUpload, upload)}
              >
                {#if cloud.busy === 'upload'}Backing up<span class="dots"></span>{:else}Back up {pUpload.length.toLocaleString('en-US')}{/if}
              </button>
            {/if}
            {#if pRestore.length}
              <button class="btn sm" disabled={!!cloud.busy} onclick={() => bulk(pRestore, (a) => restore(a, onChanged))}>
                Restore {pRestore.length.toLocaleString('en-US')} to this PC
              </button>
            {/if}
            {#if pCloud.length}
              <button class="btn sm" disabled={!!cloud.busy} onclick={() => (bulkAsk = 'drop')}>Remove {pCloud.length.toLocaleString('en-US')} from VanityCloud</button>
            {/if}
            {#if pLocal.length}
              <button class="btn sm danger" onclick={() => (bulkAsk = 'delete')}>Delete {pLocal.length.toLocaleString('en-US')} from this PC</button>
            {/if}
            <button class="btn sm" onclick={() => (picked.clear(), (bulkAsk = ''))}>Cancel</button>
          </span>
        </div>
        {#if pUpload.length > room}
          <div class="hint">Only {keys(room)} more fit in VanityCloud ({MAX_CLOUD.toLocaleString('en-US')} per account). Pick fewer to back up.</div>
        {:else if pUpload.length > PER_SEND}
          <div class="hint">They go up in {Math.ceil(pUpload.length / PER_SEND)} batches of up to {PER_SEND}, each with its own Send window. That's normal.</div>
        {/if}
        {#if bulkAsk === 'delete'}
          <div class="box warn">
            {#if lostLocal}
              <b>{keys(lostLocal)} of these {lostLocal === 1 ? 'is' : 'are'} not in VanityCloud and will be gone for good,</b> with anything held by {lostLocal ===
              1
                ? 'that address'
                : 'those addresses'}, unless you have the keys somewhere else. This can't be undone.
            {:else}
              <b>Delete {keys(pLocal.length)} from this PC?</b> Each has a copy in VanityCloud, so you can restore them later.
            {/if}
            <div class="row">
              <button class="btn sm danger" onclick={() => deleteLocal(pLocal)}>{lostLocal ? 'Delete forever' : 'Delete from this PC'}</button>
              <button class="btn sm" onclick={() => (bulkAsk = '')}>Keep them</button>
            </div>
          </div>
        {:else if bulkAsk === 'drop'}
          <div class="box warn">
            {#if lostCloud}
              <b>{keys(lostCloud)} of these {lostCloud === 1 ? 'is' : 'are'} only in VanityCloud and will be gone for good,</b> with anything held by {lostCloud ===
              1
                ? 'that address'
                : 'those addresses'}. This can't be undone.
            {:else}
              <b>Remove {keys(pCloud.length)} from VanityCloud?</b> They stay in the vault on this PC.
            {/if}
            <div class="row">
              <button class="btn sm danger" onclick={() => bulk(pCloud, drop)}
                >{lostCloud ? 'Remove forever' : 'Remove from VanityCloud'}</button
              >
              <button class="btn sm" onclick={() => (bulkAsk = '')}>Keep them</button>
            </div>
          </div>
        {/if}
        {#if bulkErr}<div class="hint bad">{bulkErr}</div>{/if}
      </div>
    {/if}
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
  .sort {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 26px;
  }
  .seg {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 11px;
    padding: 3px;
  }
  .seg button {
    font-size: 12.5px;
    font-weight: 500;
    padding: 7px 12px;
    border-radius: 8px;
    color: var(--mute);
    transition:
      background 0.2s,
      color 0.2s;
  }
  .seg button.on {
    background: rgba(255, 255, 255, 0.09);
    color: var(--text);
  }
  .sort {
    flex-wrap: wrap;
    max-width: 920px;
  }
  .look {
    height: 38px;
    width: 210px;
    margin-left: auto;
    padding: 0 13px;
    border: 1px solid var(--line);
    border-radius: 11px;
    background: var(--ink2);
    font-size: 12.5px;
    outline: 0;
    user-select: text;
    transition: border-color 0.2s;
  }
  .look:focus {
    border-color: rgba(20, 241, 149, 0.45);
  }
  .count {
    font-size: 11px;
    color: var(--dim);
  }
  .none {
    margin-top: 18px;
    color: var(--mute);
    font-size: 14.5px;
  }
  .sort + .list,
  .none + .list {
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
  .cloud {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    max-width: 920px;
    margin-top: 22px;
    padding: 12px 16px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--ink2);
  }
  .sum {
    font-size: 12px;
    color: var(--mute);
  }
  .bulk {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .calm {
    font-size: 12px;
    color: var(--green);
  }
  .hint.good {
    color: var(--green);
  }
  .info {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .tag {
    font-style: normal;
    padding: 1px 8px;
    border-radius: 999px;
    border: 1px solid rgba(20, 241, 149, 0.35);
    color: var(--green);
  }
  .tag.dim {
    border-color: rgba(245, 184, 61, 0.45);
    color: #f5b83d;
  }
  .tag.far {
    border-color: rgba(25, 211, 243, 0.4);
    color: var(--cyan);
  }
  .m button:disabled {
    opacity: 0.4;
  }
  .len {
    width: auto;
    margin-left: 0;
    padding-right: 8px;
    color: var(--mute);
  }
  .picks {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-left: auto;
  }
  .pickall {
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--mute);
    padding: 5px 11px;
    border: 1px solid var(--line2);
    border-radius: 999px;
    transition:
      color 0.2s,
      border-color 0.2s;
  }
  .pickall:hover:not(:disabled) {
    color: var(--text);
    border-color: rgba(255, 255, 255, 0.34);
  }
  .pickall:disabled {
    opacity: 0.4;
  }
  .sort + .sort {
    margin-top: 10px;
  }
  .pick {
    margin: 0 12px 0 0;
    vertical-align: -2px;
    accent-color: var(--green);
    cursor: pointer;
  }
  .item.picked {
    border-color: rgba(20, 241, 149, 0.35);
  }
  /* Stays in view at the bottom while keys are picked. */
  .bar {
    position: sticky;
    bottom: 12px;
    z-index: 5;
    max-width: 920px;
    margin-top: 16px;
    padding: 14px 18px 16px;
    border: 1px solid rgba(20, 241, 149, 0.35);
    border-radius: 16px;
    background: var(--ink2);
    box-shadow: 0 -18px 40px -12px rgb(0 0 0 / 0.6);
    animation: enter 0.25s var(--ease);
  }
  .bar-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }
  .bar-row b {
    font-weight: 500;
    font-size: 13px;
    color: var(--green);
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
