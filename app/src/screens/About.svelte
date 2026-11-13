<script lang="ts">
  import { appVersion } from '../lib/engine';

  const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  const WORDS = ['free', 'open', 'yours', 'Norway'];
  const LEN = 44;

  let version = $state('');
  let word = $state(0);
  let locked = $state(0);
  let noise = $state('');

  $effect(() => {
    appVersion().then((v) => (version = v));
  });

  // Decoration only: random characters, with one of the words locking in letter by letter.
  $effect(() => {
    let tick = 0;
    let hold = 0;
    const id = setInterval(() => {
      let s = '';
      for (let i = 0; i < LEN; i++) s += B58[(Math.random() * 58) | 0];
      noise = s;
      if (++tick % 7) return;
      if (locked < WORDS[word].length) locked++;
      else if (++hold > 6) {
        hold = 0;
        locked = 0;
        word = (word + 1) % WORDS.length;
      }
    }, 50);
    return () => clearInterval(id);
  });

  const facts = [
    ['Free', 'The app and the backup, with no catch and no paid tier.'],
    ['Open source', 'Every line is public. Read it, build it yourself, change it.'],
    ['No wallet inside', 'It never signs anything, never connects to Solana and never holds funds.'],
    ['Checked twice', 'Every find from the graphics card is rebuilt on the processor before it is kept.'],
  ];
</script>

<div class="screen">
  <div class="label">About</div>

  <div class="mark">
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <defs>
        <linearGradient id="abg" x1="0" x2="24" y1="0" y2="0" gradientUnits="userSpaceOnUse">
          <stop stop-color="#9945ff" /><stop offset=".5" stop-color="#19d3f3" /><stop offset="1" stop-color="#14f195" />
        </linearGradient>
      </defs>
      <path class="ghost" d="M2 5l5.5 14L13 5M11 5l5.5 14L22 5" />
      <path class="draw" pathLength="1" d="M2 5l5.5 14L13 5M11 5l5.5 14L22 5" />
    </svg>
    <div class="ring"></div>
    <div class="ring two"></div>
  </div>

  <h1>Your name on it. Your key in <em class="s grad">nobody's</em> hands.</h1>
  <p class="lead">
    VanityVault finds Solana addresses that start or end with the words you choose. The keys are made on your own PC and stay there, locked in
    your vault.
  </p>

  <div class="addr mono" aria-hidden="true">
    <span class="lock">{WORDS[word].slice(0, locked)}</span>{noise.slice(0, LEN - locked)}
  </div>

  <div class="facts">
    {#each facts as [title, text], i}
      <div class="fact" style="--i:{i}"><b>{title}</b>{text}</div>
    {/each}
  </div>

  <div class="foot mono">
    <span>VanityVault{version ? ` ${version}` : ''}</span>
    <span>Made in Norway by Ted K. Johansen</span>
  </div>
</div>

<style>
  .mark {
    position: relative;
    width: 84px;
    height: 84px;
    margin: 26px 0 6px;
    display: grid;
    place-items: center;
  }
  .mark svg {
    width: 56px;
    height: 56px;
    overflow: visible;
    animation: breathe 3.6s ease-in-out 1.8s infinite;
  }
  .mark path {
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .ghost {
    stroke: rgba(255, 255, 255, 0.07);
  }
  /* The mark draws itself once, left to right. */
  .draw {
    stroke: url(#abg);
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: draw 1.7s var(--ease) 0.15s forwards;
  }
  /* Two slow rings spreading out from it. */
  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 1px solid rgba(25, 211, 243, 0.5);
    opacity: 0;
    animation: ripple 3.6s ease-out 1.6s infinite;
  }
  .ring.two {
    border-color: rgba(153, 69, 255, 0.5);
    animation-delay: 3.4s;
  }
  h1 {
    margin-top: 14px;
    max-width: 15ch;
  }
  .addr {
    margin-top: 26px;
    font-size: min(2vw, 22px);
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    font-variant-ligatures: none;
  }
  .lock {
    color: var(--green);
    text-shadow: 0 0 24px rgba(20, 241, 149, 0.55);
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
    margin-top: 28px;
    max-width: 920px;
  }
  .fact {
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 16px 16px 18px;
    background: var(--ink2);
    color: var(--mute);
    font-size: 13.5px;
    position: relative;
    overflow: hidden;
    animation: rise 0.7s var(--ease) backwards;
    animation-delay: calc(0.5s + var(--i) * 0.12s);
    transition: border-color 0.3s;
  }
  .fact:hover {
    border-color: var(--line2);
  }
  /* A thin spectrum line that sweeps across the top of each card in turn. */
  .fact::before {
    content: '';
    position: absolute;
    top: -1px;
    left: 0;
    width: 40%;
    height: 1px;
    background: var(--spectrum);
    transform: translateX(-110%);
    animation: sweep 6s linear infinite;
    animation-delay: calc(1.4s + var(--i) * 1.5s);
  }
  .fact b {
    display: block;
    color: var(--text);
    font-weight: 500;
    font-size: 16px;
    margin-bottom: 4px;
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 26px;
    margin-top: 30px;
    font-size: 12px;
    color: var(--dim);
  }
  @media (max-width: 1100px) {
    .facts {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes breathe {
    50% {
      filter: drop-shadow(0 0 10px rgba(25, 211, 243, 0.55));
    }
  }
  @keyframes ripple {
    0% {
      opacity: 0.7;
      transform: scale(0.55);
    }
    70%,
    100% {
      opacity: 0;
      transform: scale(1.5);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(14px);
    }
  }
  @keyframes sweep {
    0% {
      transform: translateX(-110%);
    }
    25%,
    100% {
      transform: translateX(260%);
    }
  }
</style>
