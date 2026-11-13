<script lang="ts">
  import { engine, type About, type Hit } from '../lib/engine';
  import { big, wait, clock } from '../lib/format';
  import Addr from '../lib/Addr.svelte';
  import Chips from '../lib/Chips.svelte';

  let { about, onOpenVault }: { about: About; onOpenVault: () => void } = $props();

  const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  const LEN = 44;

  let start = $state('');
  let end = $state('');
  let anyCase = $state(false);
  let phase = $state<'idle' | 'running' | 'found'>('idle');
  let mean = $state(0);
  // The hunt list: many words looked for at once, next to the main pattern.
  let list = $state(saved());
  let listOpen = $state(!!saved().trim());
  let words = $state(0);
  let listMean = $state(0);
  let finds = $state<Hit[]>([]);
  let findCount = $state(0);
  let note = $state('');
  let err = $state('');
  let tried = $state(0);
  let rate = $state(0);
  let lastRate = $state(0);
  let elapsedMs = $state(0);
  let peak = $state(0);
  let found = $state<Hit | null>(null);
  let noise = $state('');
  let copied = $state(false);
  let onGpu = $state(false);

  // Live estimate while typing. The Rust core owns the math and the rules.
  $effect(() => {
    const s = start;
    const e = end;
    const c = anyCase;
    const l = list;
    keep(l);
    if (!s && !e && !l.trim()) {
      mean = words = listMean = 0;
      err = '';
      return;
    }
    let live = true;
    engine
      .estimate(s, e, c, l)
      .then((m) => live && (({ mean, words, listMean } = m), (err = '')))
      .catch((x) => live && ((mean = words = listMean = 0), (err = String(x))));
    return () => (live = false);
  });

  // The list is only words, so it is remembered between sessions. No keys are ever stored this way.
  function saved(): string {
    try {
      return localStorage.getItem('huntlist') ?? '';
    } catch {
      return '';
    }
  }
  function keep(text: string) {
    try {
      localStorage.setItem('huntlist', text);
    } catch {}
  }
  async function load(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    if (file.size > 200_000) return void (err = 'That file is too big for a hunt list.');
    list = await file.text();
  }

  $effect(() => {
    const offTick = engine.onTick((t) => {
      tried = t.tried;
      elapsedMs = t.elapsedMs;
      onGpu = t.gpu;
      if (t.rate > 0) {
        rate = t.rate;
        lastRate = t.rate;
        peak = Math.max(peak, t.rate);
      }
    });
    const offHit = engine.onHit((h) => {
      found = h;
      phase = 'found';
    });
    const offFind = engine.onFind((h) => {
      finds = [h, ...finds].slice(0, 3);
      findCount = h.finds;
    });
    const offEnd = engine.onEnd((n) => {
      note = `Every word on the hunt list was found at both ends. ${n} finds are in your vault.`;
      phase = 'idle';
    });
    return () => (offTick(), offHit(), offFind(), offEnd());
  });

  // The scrambling characters are decoration. Real keys stay in the core.
  $effect(() => {
    if (phase !== 'running') return;
    const n = LEN - start.length - end.length;
    const id = setInterval(() => {
      let s = '';
      for (let i = 0; i < n; i++) s += B58[(Math.random() * 58) | 0];
      noise = s;
    }, 45);
    return () => clearInterval(id);
  });

  // With no main pattern the numbers describe the next hunt list find.
  const typical = $derived(mean || listMean);
  const chance = $derived(typical ? 1 - Math.exp(-tried / typical) : 0);
  const speed = $derived(
    rate >= 1e6 ? { v: (rate / 1e6).toFixed(rate >= 1e7 ? 1 : 2), u: 'million / second' } : { v: Math.round(rate / 1e3).toString(), u: 'thousand / second' },
  );
  // The speed bar: how close the current speed is to the best seen in this hunt.
  const SEGS = 60;
  const lit = $derived(peak ? Math.round((rate / peak) * SEGS) : 0);
  const max = $derived(peak >= 1e6 ? `${(peak / 1e6).toFixed(peak >= 1e7 ? 1 : 2)} M/s` : `${Math.round(peak / 1e3)} K/s`);

  async function go() {
    if ((!start && !end && !words) || err) return;
    try {
      ({ mean, words, listMean } = await engine.start(start, end, anyCase, list));
      finds = [];
      findCount = 0;
      note = '';
      tried = 0;
      rate = 0;
      elapsedMs = 0;
      peak = 0;
      found = null;
      onGpu = about.card.state === 'ready';
      phase = 'running';
    } catch (e) {
      err = String(e);
    }
  }
  async function stop() {
    await engine.stop();
    phase = 'idle';
  }
  async function copy() {
    if (!found) return;
    await navigator.clipboard.writeText(found.address);
    copied = true;
    setTimeout(() => (copied = false), 1600);
  }
  const enter = (e: KeyboardEvent) => e.key === 'Enter' && go();
</script>

<div class="screen">
  {#if phase === 'idle'}
    <div class="label">Hunt</div>
    <h1>What should your address <em class="s">say?</em></h1>
    <p class="lead">Pick how it starts, how it ends, or both. Your PC makes fresh keys until one of them has an address that matches.</p>

    <div class="field" class:bad={!!err}>
      <label>
        <span class="label">Starts with</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input bind:value={start} placeholder="ted" maxlength="10" spellcheck="false" autocomplete="off" autofocus onkeydown={enter} />
      </label>
      <span class="gap mono">&middot;&middot;&middot;</span>
      <label class="r">
        <span class="label">Ends with</span>
        <input bind:value={end} placeholder="kj" maxlength="10" spellcheck="false" autocomplete="off" onkeydown={enter} />
      </label>
      <div class="seg">
        <button class:on={!anyCase} onclick={() => (anyCase = false)}>Exact case</button>
        <button class:on={anyCase} onclick={() => (anyCase = true)}>Any case</button>
      </div>
    </div>

    <div class="hint" class:err={!!err}>
      {#if err}
        {err}
      {:else if mean || words}
        {#if mean}
          About <b>{big(mean)}</b> tries.
          {#if lastRate}Typical wait on this PC: <b>{wait(mean / lastRate)}</b>.{/if}
          {#if start && mean > 2e9}Letters at the end are much faster to find than letters at the start.{/if}
        {/if}
        {#if words}
          Hunt list: <b>{words}</b> {words === 1 ? 'word' : 'words'}, a find about every
          <b>{lastRate ? wait(listMean / lastRate).replace('about ', '') : `${big(listMean)} tries`}</b>.
        {/if}
      {:else if note}
        {note}
      {:else}
        Letters and digits work, except 0, O, I and l. Solana addresses never contain those four.
      {/if}
    </div>

    <div class="list">
      <button class="more" onclick={() => (listOpen = !listOpen)}>
        <span>{listOpen ? '\u2212' : '+'}</span> Hunt list{words ? `, ${words} ${words === 1 ? 'word' : 'words'}` : ''}
      </button>
      {#if listOpen}
        <textarea bind:value={list} rows="5" spellcheck="false" autocomplete="off" placeholder={'moon\ndegen\nwagmi'}></textarea>
        <div class="listnote">
          <span>
            One word per line, 3 to 10 characters, up to 5000 words. Every word is looked for at both the start and the end of the address. The
            first find for each goes into your vault and the hunt carries on.
          </span>
          <label class="more">Load a text file<input type="file" accept=".txt,text/plain" hidden onchange={load} /></label>
        </div>
      {/if}
    </div>

    <button class="btn pri go" disabled={(!start && !end && !words) || !!err} onclick={go}>Start the hunt</button>
  {:else if phase === 'running'}
    <div class="label" style:color="var(--mute)">
      Hunting for
      {#if start}<span class="g">{start}</span> at the start{/if}
      {#if start && end}and{/if}
      {#if end}<span class="g">{end}</span> at the end{/if}
      {#if words}{start || end ? 'plus' : ''} <span class="g">{words}</span> list {words === 1 ? 'word' : 'words'}{/if}{anyCase ? ', any case' : ''}
    </div>
    <div class="addr mono"><span class="lock">{start}</span>{noise}<span class="lock">{end}</span></div>

    <div class="top">
      <div class="speed">{speed.v}<small>{speed.u}</small></div>
      <Chips
        cpu={!onGpu}
        gpu={onGpu}
        gpuBusy={!onGpu && about.card.state === 'starting'}
        cpuNote={onGpu ? 'checks every find' : `${about.threads} threads working`}
        gpuNote={onGpu ? about.card.name : about.card.state === 'starting' ? 'compiling shaders' : 'not in use'}
      />
    </div>
    <div class="bar">
      <div class="segs">{#each { length: SEGS } as _, i}<i class:lit={i < lit} style:--p={i / (SEGS - 1)}></i>{/each}</div>
      <span class="mono">max {peak ? max : '...'}</span>
    </div>

    <div class="stats">
      <div class="stat"><div class="label">Tried</div><b>{big(tried)}</b></div>
      <div class="stat"><div class="label">{mean ? 'Typical wait' : 'A list find every'}</div><b>{rate ? wait(typical / rate) : 'measuring'}</b></div>
      <div class="stat"><div class="label">Chance so far</div><b>{Math.round(chance * 100)}%</b></div>
      <div class="stat"><div class="label">Time</div><b>{clock(elapsedMs)}</b></div>
    </div>
    <div class="prog"><i style:width="{chance * 100}%"></i></div>
    <p class="fine">It is pure luck, so the bar is a chance, not a countdown. It can hit early or run past the end.</p>

    {#if words}
      <div class="finds">
        <div class="label">Hunt list finds, all in your vault <b>{findCount}</b></div>
        {#each finds as f (f.address)}
          <div class="find mono"><Addr address={f.address} prefixLen={f.prefixLen} suffixLen={f.suffixLen} /></div>
        {:else}
          <div class="find none">Nothing yet. Short words come first.</div>
        {/each}
      </div>
    {/if}

    <button class="btn go" onclick={stop}>Stop</button>
  {:else if found}
    <div class="label">Found</div>
    <h1>There it <em class="s grad">is.</em></h1>

    <div class="hitcard">
      <div class="addr mono sel"><Addr address={found.address} prefixLen={found.prefixLen} suffixLen={found.suffixLen} /></div>
      <div class="meta mono">
        {#if found.saved}
          <span class="g">&#10003; checked a second time, then locked in your vault</span>
        {:else}
          <span class="bad">Not saved: {found.error}</span>
        {/if}
        <span>{big(found.tries)} tries &middot; {clock(elapsedMs)}</span>
      </div>
    </div>
    {#if found.finds}<p class="fine">Plus {found.finds} hunt list {found.finds === 1 ? 'find' : 'finds'}, also in your vault.</p>{/if}

    <div class="row">
      <button class="btn pri" onclick={copy}>{copied ? 'Copied' : 'Copy address'}</button>
      <button class="btn" onclick={() => (phase = 'idle')}>Hunt again</button>
      <button class="btn" onclick={onOpenVault}>Open vault</button>
    </div>
    <p class="fine">To use it in a wallet, open the vault and show the secret key or save the key file.</p>
  {/if}
</div>

<style>
  .g {
    color: var(--green);
  }
  .bad {
    color: var(--red);
  }
  h1 {
    margin-top: 14px;
  }
  .field {
    margin-top: 34px;
    display: flex;
    align-items: flex-end;
    gap: 14px;
    flex-wrap: wrap;
    border: 1px solid var(--line2);
    border-radius: 16px;
    padding: 12px 12px 10px 22px;
    background: var(--ink3);
    max-width: 880px;
    transition:
      border-color 0.25s,
      box-shadow 0.25s;
  }
  .field:focus-within {
    border-color: rgba(20, 241, 149, 0.45);
    box-shadow: 0 0 0 4px rgba(20, 241, 149, 0.06);
  }
  .field.bad {
    border-color: rgba(255, 93, 93, 0.55);
  }
  label {
    flex: 1;
    min-width: 150px;
    display: flex;
    flex-direction: column;
  }
  label.r,
  label.r input {
    text-align: right;
  }
  input {
    width: 100%;
    background: none;
    border: 0;
    outline: 0;
    font-family: var(--mono);
    font-size: 26px;
    height: 44px;
    caret-color: var(--green);
    user-select: text;
  }
  input::placeholder {
    color: rgba(255, 255, 255, 0.13);
  }
  .gap {
    color: var(--dim);
    font-size: 22px;
    padding-bottom: 8px;
  }
  .seg {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 11px;
    padding: 3px;
    margin: 0 0 3px 8px;
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
  .hint {
    margin: 16px 4px 0;
    color: var(--mute);
    font-size: 14.5px;
    min-height: 22px;
    max-width: 880px;
  }
  .hint b {
    color: var(--text);
    font-weight: 500;
  }
  .hint.err {
    color: var(--red);
  }
  .list {
    margin-top: 14px;
    max-width: 880px;
  }
  .more {
    font-size: 13.5px;
    color: var(--mute);
    cursor: pointer;
    white-space: nowrap;
  }
  .more:hover {
    color: var(--text);
  }
  .more span {
    display: inline-block;
    width: 14px;
    font-family: var(--mono);
    color: var(--green);
  }
  textarea {
    display: block;
    width: 100%;
    margin-top: 10px;
    padding: 12px 16px;
    border: 1px solid var(--line2);
    border-radius: 14px;
    background: var(--ink3);
    outline: 0;
    resize: vertical;
    font-family: var(--mono);
    font-size: 14px;
    line-height: 1.6;
    color: var(--text);
    caret-color: var(--green);
    user-select: text;
  }
  textarea:focus {
    border-color: rgba(20, 241, 149, 0.45);
  }
  textarea::placeholder {
    color: rgba(255, 255, 255, 0.16);
  }
  .listnote {
    display: flex;
    gap: 24px;
    justify-content: space-between;
    align-items: flex-start;
    margin-top: 8px;
    font-size: 13px;
    color: var(--dim);
  }
  .listnote .more {
    text-decoration: underline;
    text-underline-offset: 3px;
    text-decoration-color: var(--line2);
  }
  .finds {
    margin-top: 18px;
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 13px 14px;
  }
  .finds .label b {
    color: var(--green);
    font-weight: 500;
    margin-left: 8px;
  }
  .find {
    margin-top: 7px;
    font-size: 13px;
    color: var(--mute);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: text;
    animation: enter 0.4s var(--ease);
  }
  .find.none {
    color: var(--dim);
  }
  .go {
    margin-top: 30px;
    min-width: 190px;
  }
  .addr {
    font-size: min(2.55vw, 30px);
    color: var(--dim);
    white-space: nowrap;
    margin-top: 18px;
    font-variant-ligatures: none;
  }
  .addr.sel {
    color: var(--text);
    user-select: text;
    margin: 0;
  }
  .lock {
    color: var(--green);
    text-shadow: 0 0 24px rgba(20, 241, 149, 0.55);
  }
  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-top: 26px;
  }
  .speed {
    font-size: clamp(60px, 8.4vw, 108px);
    font-weight: 500;
    letter-spacing: -0.06em;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .speed small {
    font-size: 0.2em;
    letter-spacing: 0;
    color: var(--mute);
    font-weight: 400;
    margin-left: 12px;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 18px 0 22px;
  }
  .segs {
    flex: 1;
    display: flex;
    gap: 3px;
    height: 14px;
  }
  .segs i {
    flex: 1;
    border-radius: 1.5px;
    background: rgba(255, 255, 255, 0.07);
    transition: background-color 0.25s;
  }
  /* Each lit segment shows its own slice of the spectrum. */
  .segs i.lit {
    background: var(--spectrum);
    background-size: 6000% 100%;
    background-position: calc(var(--p) * 100%) 0;
  }
  .bar span {
    font-size: 12.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--mute);
    white-space: nowrap;
    min-width: 128px;
    text-align: right;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }
  .stat {
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 13px 14px;
  }
  .stat b {
    font-weight: 500;
    font-size: 17px;
    font-variant-numeric: tabular-nums;
  }
  .prog {
    height: 3px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.07);
    margin-top: 22px;
    overflow: hidden;
  }
  .prog i {
    display: block;
    height: 100%;
    background: var(--spectrum);
    transition: width 0.3s linear;
  }
  .fine {
    color: var(--dim);
    font-size: 13px;
    margin-top: 10px;
  }
  .hitcard {
    margin-top: 30px;
    border: 1px solid var(--line);
    border-radius: 20px;
    padding: 26px 28px;
    background: linear-gradient(180deg, #0d1014, #090b0e);
    position: relative;
    animation: land 0.7s var(--ease);
  }
  .hitcard::before {
    content: '';
    position: absolute;
    left: 10%;
    right: 10%;
    top: -1px;
    height: 1px;
    background: var(--spectrum);
  }
  @keyframes land {
    from {
      opacity: 0;
      transform: scale(0.97) translateY(12px);
    }
  }
  .meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 16px;
    font-size: 12px;
    color: var(--mute);
  }
  .row {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin: 26px 0 8px;
  }
</style>
