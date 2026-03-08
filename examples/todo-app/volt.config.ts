import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'Todo App',
  version: '0.1.0',
  backend: './src/backend.ts',
  window: {
    width: 500,
    height: 700,
    title: 'Volt Todo',
    minWidth: 400,
    minHeight: 500,
  },
  permissions: ['db'],
});
