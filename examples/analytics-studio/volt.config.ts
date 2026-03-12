import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'Analytics Studio',
  version: '0.1.0',
  backend: './src/backend.ts',
  permissions: ['fs'],
  window: {
    width: 1320,
    height: 860,
    minWidth: 1180,
    minHeight: 720,
    title: 'Volt Analytics Studio',
  },
});
