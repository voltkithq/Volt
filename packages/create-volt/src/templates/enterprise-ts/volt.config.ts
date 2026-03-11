import { defineConfig } from 'volt-framework';

export default defineConfig({
  name: 'My Volt Enterprise App',
  version: '0.1.0',
  backend: './src/backend.ts',
  permissions: ['secureStorage', 'fs', 'dialog', 'http', 'db', 'shell'],
  window: {
    width: 1080,
    height: 760,
    minWidth: 900,
    minHeight: 620,
    title: 'My Volt Enterprise App',
    icon: './public/favicon.png',
  },
  package: {
    identifier: 'com.example.my-volt-enterprise-app',
    windows: {
      installMode: 'perMachine',
      silentAllUsers: true,
    },
    enterprise: {
      generateAdmx: true,
      includeDocsBundle: true,
    },
  },
});
