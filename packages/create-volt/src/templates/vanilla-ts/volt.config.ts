import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'My Volt App',
  version: '0.1.0',
  backend: './src/backend.ts',
  window: {
    width: 800,
    height: 600,
    title: 'My Volt App',
    icon: './public/favicon.png',
  },
});
