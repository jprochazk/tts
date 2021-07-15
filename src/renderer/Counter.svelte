<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let start: number = 0;
  export let step: number = 1;
  let value: number = start;

  const emit = createEventDispatcher<{
    change: { value: number };
  }>();
  const increment = () => {
    value += step;
    emit("change", { value });
  };
  const decrement = () => {
    value -= step;
    emit("change", { value });
  };
</script>

<div>
  <button on:click={increment}>+</button>
  <button on:click={decrement}>-</button>
</div>
