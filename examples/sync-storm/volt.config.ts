import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'Sync Storm',
  version: '0.1.0',
  backend: './src/backend.ts',
  permissions: ['fs'],
  window: {
    width: 1280,
    height: 840,
    minWidth: 1080,
    minHeight: 720,
    title: 'Volt Sync Storm',
  },
});
