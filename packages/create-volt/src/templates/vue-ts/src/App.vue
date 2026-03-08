<script setup lang="ts">
import { ref } from 'vue';

const count = ref(0);
</script>

<template>
  <div class="app">
    <h1>Welcome to Volt!</h1>
    <p>Edit <code>src/App.vue</code> and save to see changes.</p>
    <button @click="count++">
      Count: {{ count }}
    </button>
  </div>
</template>

<style scoped>
.app {
  max-width: 480px;
  margin: 0 auto;
  padding: 2rem;
  text-align: center;
}
</style>
