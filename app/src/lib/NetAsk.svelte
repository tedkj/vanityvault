<script lang="ts">
  // Full transparency: the core shows every request here before it leaves the PC. `raw` is the
  // exact text it will send, and Send sends that same text, byte for byte.
  import { engine, type NetAsk } from './engine';

  let queue = $state<NetAsk[]>([]);
  let trust = $state(false);
  let view = $state<'explained' | 'raw'>('explained');
  const cur = $derived(queue[0]);
  const body = $derived(cur ? (JSON.parse(cur.raw) as Record<string, unknown>) : {});
  const bytes = $derived(cur ? new TextEncoder().encode(cur.raw).length : 0);

  $effect(() => engine.onNetAsk((a) => (queue = [...queue, a])));

  const plural = (n: number) => `${n} ${n === 1 ? 'key' : 'keys'}`;
  function what(a: NetAsk, b: Record<string, unknown>): [string, string] {
    const n = (k: string) => (Array.isArray(b[k]) ? (b[k] as unknown[]).length : 0);
    switch (a.path) {
      case 'prelogin': return ['Ask for your password settings', "Your username only. The VanityCloud server answers with the salt and settings this PC needs to turn your password into keys. It answers the same way for names that don't exist."];
      case 'register': return ['Create your backup account', 'Your selected username and keys made on this PC from your VanityCloud password. The password itself is not in here.'];
      case 'login': return ['Sign in', 'A proof made on this PC from your VanityCloud password. It opens nothing: the VanityCloud server only compares it with what it has.'];
      case 'recover/start': return ['Sign in with your VanityCloud recovery key', 'A proof made on this PC from your VanityCloud recovery key. It opens nothing.'];
      case 'recover/finish': return ['Set your new VanityCloud password', 'Keys made on this PC from your new VanityCloud password. The password itself is not in here.'];
      case 'password': return ['Change your VanityCloud password', 'A proof of your current VanityCloud password and keys made on this PC from the new one. Neither password is in here.'];
      case 'account/delete': return ['Delete your backup account', 'A proof of your VanityCloud password. The VanityCloud server then deletes the account and every box in it.'];
      case 'entries/put': return [`Back up ${plural(n('entries'))}`, 'Each key travels as a locked box. Only your password opens it.'];
      case 'entries/delete': return [`Remove ${plural(n('ids'))} from the backup`, 'The box ids only. Your vault on this PC is not touched.'];
      default: return ['A request to the VanityCloud server', 'The full request is below.'];
    }
  }
  const NOTES: Record<string, string> = {
    username: 'The username you selected.',
    account_id: 'A random number this app picked.',
    salt: 'Random. Needed to turn your VanityCloud password into keys.',
    kdf: 'How hard your PC works on your VanityCloud password.',
    auth_key: 'Proof that you know your VanityCloud password. Made on this PC, opens nothing.',
    new_auth_key: 'The same kind of proof, for the new VanityCloud password.',
    recovery_auth: 'Proof that you have your VanityCloud recovery key. Opens nothing.',
    wrapped_key: 'The key that locks your boxes, itself locked. Only your VanityCloud password opens it.',
    wrapped_key_recovery: 'The same key, locked a second way. Only your VanityCloud recovery key opens it.',
    entries: 'Locked boxes. The id says nothing about the key, the nonce is random, and ct is the locked key with its address.',
    ids: 'Which boxes to remove. The ids say nothing about the keys.',
  };

  function answer(ok: boolean) {
    if (!cur) return;
    engine.netAnswer(cur.id, ok, ok && trust);
    queue = queue.slice(1);
    view = 'explained';
    // Once trusted, the core stops asking; anything already waiting goes the same way.
    if (ok && trust) {
      queue.forEach((a) => engine.netAnswer(a.id, true, true));
      queue = [];
    }
  }
  const onKey = (e: KeyboardEvent) => cur && e.key === 'Escape' && answer(false);
</script>

<svelte:window onkeydown={onKey} />

{#if cur}
  {@const [title, why] = what(cur, body)}
  <div class="veil">
    <div class="ask" role="dialog" aria-modal="true" aria-labelledby="ask-t">
      <div class="top">
        <span class="badge">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12Z" /><circle cx="12" cy="12" r="3" /></svg>
          Full transparency
        </span>
        <span class="held"><i></i>Nothing sent yet</span>
      </div>
      <h2 id="ask-t">This is exactly what the app is about to send</h2>
      <p class="intro">VanityVault shows you every message that carries data before it leaves your PC, byte for byte. Nothing goes out until you click Send.</p>

      <div class="mid">
        <div class="req">
          <div class="req-h"><b>{title}</b><p>{why}</p></div>
          <div class="tabs" role="tablist">
            <button role="tab" aria-selected={view === 'explained'} class:on={view === 'explained'} onclick={() => (view = 'explained')}>Explained</button>
            <button role="tab" aria-selected={view === 'raw'} class:on={view === 'raw'} onclick={() => (view = 'raw')}>Raw &middot; {bytes.toLocaleString('en')} bytes</button>
          </div>
          {#if view === 'explained'}
            <div class="code">
              <div class="line"><span class="m">POST</span> {cur.url}</div>
              {#if cur.signedIn}<div class="line dim">+ your session pass: a random number the VanityCloud server gave out when you signed in</div>{/if}
              <pre>{JSON.stringify(body, null, 2)}</pre>
            </div>
            <dl class="notes">
              {#each Object.keys(body).filter((k) => NOTES[k]) as k (k)}
                <div><dt>{k}</dt><dd>{NOTES[k]}</dd></div>
              {/each}
            </dl>
          {:else}
            <div class="code">
              <pre class="raw">POST {cur.url}
Content-Type: application/json
User-Agent: {cur.userAgent}{#if cur.signedIn}
Authorization: Bearer (your session pass, hidden){/if}

<span class="b">{cur.raw}</span></pre>
            </div>
            <p class="raw-note">The last block is the message itself, exactly as it is sent. The lines above it go along with every request.</p>
          {/if}
        </div>
      </div>

      <label class="tick"><input type="checkbox" bind:checked={trust} /><span>Stop showing these until VanityVault closes</span></label>
      <div class="row">
        <button class="btn" onclick={() => answer(false)}>Don't send</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button class="btn pri" onclick={() => answer(true)} autofocus>Send it</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .veil {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 28px;
    background: rgb(4 5 7 / 0.72);
    backdrop-filter: blur(6px);
    animation: fade 0.2s ease;
  }
  .ask {
    width: min(740px, 100%);
    max-height: calc(100vh - 56px);
    display: flex;
    flex-direction: column;
    padding: 24px 28px;
    border: 1px solid var(--line2);
    border-radius: 20px;
    background: var(--ink2);
    box-shadow: 0 30px 80px -20px rgb(0 0 0 / 0.7);
    animation: rise 0.3s var(--ease);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px 5px 9px;
    border: 1px solid rgb(20 241 149 / 0.35);
    border-radius: 999px;
    background: rgb(20 241 149 / 0.07);
    color: var(--green);
    font-family: var(--mono);
    font-size: 12px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .badge svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
  }
  .held {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--mute);
  }
  .held i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--cyan);
    box-shadow: 0 0 10px var(--cyan);
    animation: blink 1.6s ease-in-out infinite;
  }
  h2 {
    margin: 16px 0 6px;
    font-size: 23px;
    font-weight: 500;
    letter-spacing: -0.03em;
  }
  .intro {
    margin: 0 0 16px;
    color: var(--mute);
    font-size: 14.5px;
    max-width: 64ch;
  }
  /* The request scrolls; the header and the buttons stay in view. */
  .mid {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    margin-right: -10px;
    padding-right: 10px;
  }
  .req {
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 16px;
    background: rgb(255 255 255 / 0.015);
  }
  .req-h b {
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.01em;
  }
  .req-h p {
    margin: 4px 0 0;
    color: var(--mute);
    font-size: 13.5px;
  }
  .tabs {
    display: flex;
    gap: 6px;
    margin: 14px 0 10px;
  }
  .tabs button {
    padding: 5px 12px;
    border: 1px solid var(--line);
    border-radius: 999px;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--mute);
    transition: border-color 0.2s, color 0.2s;
  }
  .tabs button:hover {
    border-color: var(--line2);
    color: var(--text);
  }
  .tabs button.on {
    border-color: var(--line2);
    background: var(--ink3);
    color: var(--text);
  }
  .code {
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--ink);
    padding: 12px 14px;
    font-family: var(--mono);
    font-size: 12px;
  }
  .line {
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .line.dim {
    color: var(--dim);
    margin-top: 4px;
  }
  .m {
    color: var(--green);
  }
  pre {
    margin: 10px 0 0;
    padding-top: 10px;
    border-top: 1px solid var(--line);
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--mute);
    font: inherit;
    line-height: 1.6;
    user-select: text;
  }
  pre.raw {
    margin: 0;
    padding: 0;
    border: 0;
    color: var(--dim);
  }
  pre.raw .b {
    color: var(--text);
  }
  .raw-note {
    margin: 8px 0 0;
    color: var(--dim);
    font-size: 12.5px;
  }
  .notes {
    margin: 14px 0 0;
    display: grid;
    gap: 7px;
  }
  .notes div {
    display: grid;
    grid-template-columns: 170px 1fr;
    gap: 14px;
    font-size: 13.5px;
  }
  .notes dt {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--green);
    padding-top: 1px;
  }
  .notes dd {
    margin: 0;
    color: var(--mute);
  }
  .tick {
    margin-top: 16px;
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 16px;
  }
  @keyframes fade {
    from { opacity: 0; }
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(12px) scale(0.98); }
  }
  @keyframes blink {
    50% { opacity: 0.35; }
  }
</style>
