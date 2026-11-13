<script lang="ts">
  import { FAQ } from '../lib/faq';

  // The four secrets appear as the same labels the app uses everywhere else.
  const NAMES = /(vault password|vault recovery key|VanityCloud password|VanityCloud recovery key)/i;
  const parts = (text: string) =>
    text.split(NAMES).map((t, i) => ({ t, kind: i % 2 ? (t.toLowerCase().startsWith('vault') ? 'vault' : 'cloud') : '' }));
</script>

<div class="screen">
  <div class="label">FAQ</div>
  <h1>Questions, <em class="s">answered.</em></h1>
  <p class="lead">The short version of what people ask first.</p>

  {#each FAQ as sec (sec.title)}
    <div class="label sec">{sec.title}</div>
    <div class="qa">
      {#each sec.items as { q, a } (q)}
        <details>
          <summary><span>{q}</span><i aria-hidden="true"></i></summary>
          <div class="a">
            {#each a as p}<p>{#each parts(p) as { t, kind }}{#if kind}<span class="kt {kind}">{t}</span>{:else}{t}{/if}{/each}</p>{/each}
          </div>
        </details>
      {/each}
    </div>
  {/each}
</div>

<style>
  h1 {
    margin-top: 14px;
  }
  .sec {
    margin: 30px 0 10px;
  }
  .qa {
    max-width: 860px;
    display: grid;
    gap: 8px;
  }
  details {
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--ink2);
    transition: border-color 0.2s;
  }
  details[open],
  details:hover {
    border-color: var(--line2);
  }
  summary {
    list-style: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 15px 18px;
    font-size: 15.5px;
    font-weight: 500;
    letter-spacing: -0.01em;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary i {
    flex: none;
    position: relative;
    width: 26px;
    height: 26px;
    border: 1px solid var(--line2);
    border-radius: 50%;
  }
  summary i::before,
  summary i::after {
    content: '';
    position: absolute;
    left: 50%;
    top: 50%;
    width: 10px;
    height: 1.5px;
    background: var(--text);
    transform: translate(-50%, -50%);
  }
  summary i::after {
    transform: translate(-50%, -50%) rotate(90deg);
    transition: transform 0.25s var(--ease);
  }
  details[open] summary i::after {
    transform: translate(-50%, -50%) rotate(0deg);
  }
  details[open] summary i {
    border-color: var(--green);
  }
  .a {
    padding: 0 18px 16px;
    max-width: 76ch;
  }
  .a p {
    margin: 0 0 10px;
    color: var(--mute);
    font-size: 14.5px;
    line-height: 1.6;
  }
  .a p:last-child {
    margin-bottom: 0;
  }
</style>
