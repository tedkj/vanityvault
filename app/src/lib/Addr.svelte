<script lang="ts">
  let { address, prefixLen, suffixLen }: { address: string; prefixLen: number; suffixLen: number } = $props();
  const cut = $derived(address.length - suffixLen);
</script>

<b>{address.slice(0, prefixLen)}</b>{address.slice(prefixLen, cut)}<b>{address.slice(cut)}</b>

<style>
  b {
    color: var(--green);
    font-weight: 500;
  }
</style>
