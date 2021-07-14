<script lang="typescript">
  let count: number = 0;
  $: large = count > 10;
  $: double = count * 2;
</script>

<span>{count} * 2 = {double}, which is {large ? "large" : "small"}</span><br>
<button on:click={() => count += 1}>+</button>

<style global>
  @tailwind base;
  @tailwind components;
  @tailwind utilities;
</style>