<script lang="ts">
  import Counter from "./Counter.svelte";

  let count: number = 20;
  $: double = count * 2;
</script>

<div class="container">
  <Counter start={count} step={2} on:change={(e) => (count = e.detail.value)} />
  <div>
    <span>Count: {count}</span><br />
    <span>Double: {double}</span>
  </div>
</div>

<style global>
  @tailwind base;
  @tailwind components;
  @tailwind utilities;
</style>
