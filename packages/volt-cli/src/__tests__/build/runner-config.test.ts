import { describe, expect, it } from 'vitest';

import { __testOnly } from '../../commands/build.js';

describe('build runner config payload helpers', () => {
  it('builds runner config payload with permissions, plugin settings, and window options', () => {
    const payload = __testOnly.buildRunnerConfigPayload({
      name: 'IPC Demo',
      devtools: true,
      permissions: ['clipboard'],
      plugins: {
        enabled: ['acme.search'],
        grants: {
          'acme.search': ['fs', 'http'],
        },
        pluginDirs: ['./plugins'],
        limits: {
          activationTimeoutMs: 10_000,
          deactivationTimeoutMs: 5_000,
          callTimeoutMs: 30_000,
          maxPlugins: 32,
          heartbeatIntervalMs: 1_500,
          heartbeatTimeoutMs: 900,
        },
        spawning: {
          strategy: 'lazy',
          idleTimeoutMs: 300_000,
          preSpawn: ['acme.search'],
        },
      },
      runtime: {
        poolSize: 3,
      },
      runtimePoolSize: 3,
      window: {
        width: 980,
        height: 760,
        title: 'Volt IPC Demo',
      },
    });

    expect(payload).toMatchObject({
      name: 'IPC Demo',
      devtools: true,
      permissions: ['clipboard'],
      plugins: {
        enabled: ['acme.search'],
        grants: {
          'acme.search': ['fs', 'http'],
        },
        pluginDirs: ['./plugins'],
        limits: {
          activationTimeoutMs: 10_000,
          deactivationTimeoutMs: 5_000,
          callTimeoutMs: 30_000,
          maxPlugins: 32,
          heartbeatIntervalMs: 1_500,
          heartbeatTimeoutMs: 900,
        },
        spawning: {
          strategy: 'lazy',
          idleTimeoutMs: 300_000,
          preSpawn: ['acme.search'],
        },
      },
      runtime: {
        poolSize: 3,
      },
      runtimePoolSize: 3,
      window: {
        width: 980,
        height: 760,
        title: 'Volt IPC Demo',
      },
    });
  });
});
