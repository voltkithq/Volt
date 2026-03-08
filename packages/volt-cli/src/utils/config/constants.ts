import type { Permission, VoltConfig } from 'voltkit';

export const DEFAULT_CONFIG: VoltConfig = {
  name: 'Volt App',
  version: '0.1.0',
  window: {
    width: 800,
    height: 600,
    title: 'Volt App',
  },
};

export const VALID_PERMISSIONS: Permission[] = [
  'clipboard',
  'notification',
  'dialog',
  'fs',
  'db',
  'menu',
  'shell',
  'http',
  'globalShortcut',
  'tray',
  'secureStorage',
];

export const CONFIG_FILES = [
  'volt.config.ts',
  'volt.config.js',
  'volt.config.mjs',
];
