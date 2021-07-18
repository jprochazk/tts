<script lang="ts">
  import Counter from "./Counter.svelte";
  import CloudSyncIcon from "material-icons/CloudSyncOutline.svelte";

  let count: number = 20;
  $: double = count * 2;

  async function test() {
    console.log(await window.api.test());
  }
</script>

<div class="container">
  <Counter start={count} step={2} on:change={(e) => (count = e.detail.value)} />
  <div>
    <span>Count: {count}</span><br />
    <span>Double: {double}</span>
  </div>
  <button on:click={test}>
    <CloudSyncIcon />
  </button>
</div>

<style global>
  @tailwind base;
  @tailwind components;
  @tailwind utilities;
</style>
