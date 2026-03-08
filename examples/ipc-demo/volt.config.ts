import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'IPC Demo',
  version: '0.1.0',
  backend: './src/backend.ts',
  window: {
    width: 980,
    height: 760,
    title: 'Volt IPC Demo',
    minWidth: 860,
    minHeight: 640,
  },
  permissions: ['clipboard', 'db', 'menu', 'globalShortcut', 'tray', 'secureStorage'],
});
