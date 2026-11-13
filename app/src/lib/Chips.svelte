<script lang="ts">
  // Decoration: a processor and a graphics card with data running in and out while they work.
  // `gpuBusy` is the one-time shader compile: the card sits shaded behind an estimate and its caption pulses.
  let { cpu, gpu, gpuBusy, cpuNote, gpuNote }: { cpu: boolean; gpu: boolean; gpuBusy: boolean; cpuNote: string; gpuNote: string } = $props();

  const side = [46, 62, 78, 94];
  const upDown = [80, 100, 120];
  const blades = [0, 1, 2, 3, 4, 5, 6].map((i) => (i * 360) / 7);
  const fingers = Array.from({ length: 14 }, (_, i) => 62 + i * 8);
  const lanes = [70, 102, 134, 166];
</script>

<div class="chips">
  <figure class:on={cpu}>
    <svg viewBox="0 0 200 140" aria-hidden="true">
      <defs>
        <linearGradient id="chipg" x1="0" x2="200" y1="0" y2="0" gradientUnits="userSpaceOnUse">
          <stop stop-color="#9945ff" /><stop offset=".5" stop-color="#19d3f3" /><stop offset="1" stop-color="#14f195" />
        </linearGradient>
      </defs>
      {#each side as y, i}
        <path class="wire" d="M0 {y}H58" /><path class="wire" d="M142 {y}H200" />
        <path class="pulse in" style="--i:{i}" d="M0 {y}H58" />
        <path class="pulse out" style="--i:{i + 2}" d="M142 {y}H200" />
        <rect class="pin" x="52" y={y - 2.5} width="6" height="5" rx="1" /><rect class="pin" x="142" y={y - 2.5} width="6" height="5" rx="1" />
      {/each}
      {#each upDown as x, i}
        <path class="wire" d="M{x} 0V28" /><path class="wire" d="M{x} 112V140" />
        <path class="pulse in" style="--i:{i + 1}" d="M{x} 0V28" />
        <path class="pulse out" style="--i:{i + 3}" d="M{x} 112V140" />
        <rect class="pin" x={x - 2.5} y="22" width="5" height="6" rx="1" /><rect class="pin" x={x - 2.5} y="112" width="5" height="6" rx="1" />
      {/each}
      <rect class="body" x="58" y="28" width="84" height="84" rx="10" />
      <rect class="die" x="78" y="48" width="44" height="44" rx="6" />
      <text x="100" y="74">CPU</text>
    </svg>
    <figcaption>{cpuNote}</figcaption>
  </figure>

  <figure class:on={gpu} class:busy={gpuBusy}>
    {#if gpuBusy}<div class="eta"><span class="label">ETA</span>2&ndash;4 minutes</div>{/if}
    <svg viewBox="0 0 240 140" aria-hidden="true">
      {#each lanes as x, i}
        <path class="wire" d="M{x} 112V140" />
        <path class="pulse {i % 2 ? 'out' : 'in'}" style="--i:{i}" d="M{x} {i % 2 ? 112 : 140}V{i % 2 ? 140 : 112}" />
      {/each}
      <rect class="pin" x="8" y="22" width="6" height="92" rx="2" />
      <rect class="body" x="18" y="28" width="212" height="76" rx="10" />
      {#each fingers as x}<rect class="pin" x={x} y="104" width="5" height="8" rx="1" />{/each}
      {#each [78, 170] as cx}
        <circle class="ring" {cx} cy="66" r="29" />
        <g transform="translate({cx} 66)">
          <g class="fan">
            {#each blades as a}<path class="blade" transform="rotate({a})" d="M0 -5C9 -10 17 -18 8 -25C-1 -22 -4 -13 0 -5Z" />{/each}
          </g>
          <circle class="hub" r="5" />
        </g>
      {/each}
      <text x="124" y="70">GPU</text>
    </svg>
    <figcaption class:busy-pulse={gpuBusy}>{gpuNote}</figcaption>
  </figure>
</div>

<style>
  .chips {
    display: flex;
    gap: 18px;
    align-items: flex-end;
  }
  figure {
    opacity: 0.38;
    transition: opacity 0.5s;
  }
  figure {
    position: relative;
  }
  figure.on,
  figure.busy {
    opacity: 1;
  }
  .busy svg {
    opacity: 0.22;
  }
  .busy text {
    opacity: 0;
  }
  .eta {
    position: absolute;
    inset: 0 0 26px;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 19px;
    font-weight: 500;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
  svg {
    display: block;
    height: 124px;
    overflow: visible;
  }
  figcaption {
    font-family: var(--mono);
    font-size: 12.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--mute);
    text-align: center;
    margin-top: 8px;
  }
  .on figcaption {
    color: var(--mute);
  }
  .wire {
    stroke: rgba(255, 255, 255, 0.1);
    stroke-width: 1;
    fill: none;
  }
  .pin {
    fill: rgba(255, 255, 255, 0.22);
  }
  .body {
    fill: var(--ink3);
    stroke: rgba(255, 255, 255, 0.16);
  }
  .die {
    fill: #07090c;
    stroke: url(#chipg);
    stroke-width: 1.2;
  }
  .on .die {
    animation: glow 1.6s ease-in-out infinite;
  }
  text {
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    fill: var(--mute);
    text-anchor: middle;
  }
  .ring {
    fill: #07090c;
    stroke: rgba(255, 255, 255, 0.14);
  }
  .blade {
    fill: rgba(255, 255, 255, 0.2);
  }
  .on .blade {
    fill: rgba(25, 211, 243, 0.55);
  }
  .hub {
    fill: var(--ink3);
    stroke: rgba(255, 255, 255, 0.25);
  }
  .fan {
    transform-origin: 0 0;
  }
  .on .fan {
    animation: spin 0.9s linear infinite;
  }
  /* A short bright dash that travels along each wire: violet going in, green coming out. */
  .pulse {
    fill: none;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-dasharray: 7 60;
    stroke-dashoffset: 7;
    opacity: 0;
  }
  .pulse.in {
    stroke: var(--violet);
  }
  .pulse.out {
    stroke: var(--green);
  }
  .on .pulse {
    opacity: 1;
    animation: flow 1.1s linear infinite;
    animation-delay: calc(var(--i) * -0.27s);
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -60;
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes glow {
    50% {
      filter: drop-shadow(0 0 7px rgba(20, 241, 149, 0.75));
    }
  }
</style>
