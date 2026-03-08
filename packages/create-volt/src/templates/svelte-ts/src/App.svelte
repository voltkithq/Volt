<script lang="ts">
  let count = $state(0);
</script>

<div class="app">
  <h1>Welcome to Volt!</h1>
  <p>Edit <code>src/App.svelte</code> and save to see changes.</p>
  <button onclick={() => count++}>
    Count: {count}
  </button>
</div>

<style>
  .app {
    max-width: 480px;
    margin: 0 auto;
    padding: 2rem;
    text-align: center;
  }
</style>
