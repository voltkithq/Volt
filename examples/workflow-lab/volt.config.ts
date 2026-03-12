import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'Workflow Lab',
  version: '0.1.0',
  backend: './src/backend.ts',
  permissions: ['fs'],
  window: {
    width: 1300,
    height: 860,
    minWidth: 1120,
    minHeight: 740,
    title: 'Volt Workflow Lab',
  },
});
