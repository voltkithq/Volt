import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'File Explorer',
  version: '0.1.0',
  backend: './src/backend.ts',
  permissions: ['fs', 'dialog'],
  window: {
    width: 900,
    height: 600,
    title: 'File Explorer',
  },
});
