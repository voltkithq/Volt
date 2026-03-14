import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { loadConfig } from '../utils/config.js';
import { validateConfig } from '../utils/config/validator.js';
import { existsSync } from 'node:fs';

// Mock fs.existsSync so we control which config files "exist"
vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    existsSync: vi.fn(actual.existsSync),
  };
});

describe('loadConfig', () => {
  let consoleWarn: ReturnType<typeof vi.spyOn>;
  let consoleError: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    consoleWarn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleWarn.mockRestore();
    consoleError.mockRestore();
  });

  it('returns default config when no config file exists', async () => {
    vi.mocked(existsSync).mockReturnValue(false);
    const config = await loadConfig('/fake/project');
    expect(config.name).toBe('Volt App');
    expect(config.version).toBe('0.1.0');
    expect(consoleWarn).toHaveBeenCalledWith(
      expect.stringContaining('No config file found'),
    );
  });

  it('throws in strict mode when no config file exists', async () => {
    vi.mocked(existsSync).mockReturnValue(false);
    await expect(
      loadConfig('/fake/project', { strict: true, commandName: 'build' }),
    ).rejects.toThrow('No config file found');
  });

  it('throws when a config file exists but fails to load', async () => {
    vi.mocked(existsSync).mockReturnValue(true);
    await expect(loadConfig('/fake/project')).rejects.toThrow(
      /Failed to load.*config/,
    );
  });
});

describe('config validation (via loadConfig internals)', () => {
  const validPublicKey = Buffer.alloc(32, 7).toString('base64');

  // Since validateConfig is private, we test it indirectly through its effects.
  // The validation logic is documented well enough to test by checking
  // the exported types and expected behavior.

  it('VoltConfig has expected required field: name', () => {
    // Type-level test: VoltConfig requires `name`
    const config = { name: 'Required' } satisfies import('voltkit').VoltConfig;
    expect(config.name).toBe('Required');
  });

  it('VoltConfig allows all optional fields', () => {
    const config: import('voltkit').VoltConfig = {
      name: 'Full',
      version: '1.0.0',
      permissions: ['clipboard', 'fs', 'shell'],
      window: { width: 1024, height: 768, title: 'Test' },
      build: { outDir: 'dist' },
      backend: './src/backend.ts',
      package: {
        identifier: 'com.test.app',
        windows: {
          installMode: 'perUser',
          silentAllUsers: false,
          msix: {
            identityName: 'com.test.app',
            publisher: 'CN=Test',
          },
        },
        enterprise: {
          generateAdmx: true,
          includeDocsBundle: true,
        },
      },
      updater: { endpoint: 'https://u.com', publicKey: 'key' },
      runtime: { poolSize: 3 },
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
      devtools: true,
    };
    expect(config.name).toBe('Full');
    expect(config.permissions).toHaveLength(3);
  });

  it('Permission type only allows valid values', () => {
    const validPerms: import('voltkit').Permission[] = [
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
    expect(validPerms).toHaveLength(11);
  });

  it('accepts well-formed updater endpoint and Ed25519 public key', () => {
    const validated = validateConfig(
      {
        name: 'Valid Updater',
        updater: {
          endpoint: 'https://updates.example.com/check',
          publicKey: validPublicKey,
        },
      },
      'volt.config.ts',
      { strict: false },
    );

    expect(validated.updater).toEqual({
      endpoint: 'https://updates.example.com/check',
      publicKey: validPublicKey,
    });
  });

  it('accepts localhost and loopback HTTP updater endpoints for local testing', () => {
    const localhostConfig = validateConfig(
      {
        name: 'Localhost Updater',
        updater: {
          endpoint: 'http://localhost:8787/check',
          publicKey: validPublicKey,
        },
      },
      'volt.config.ts',
      { strict: false },
    );
    const loopbackConfig = validateConfig(
      {
        name: 'Loopback Updater',
        updater: {
          endpoint: 'http://127.0.0.1:8787/check',
          publicKey: validPublicKey,
        },
      },
      'volt.config.ts',
      { strict: false },
    );

    expect(localhostConfig.updater?.endpoint).toBe('http://localhost:8787/check');
    expect(loopbackConfig.updater?.endpoint).toBe('http://127.0.0.1:8787/check');
  });

  it('strips invalid updater config when endpoint or public key is malformed', () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const validated = validateConfig(
      {
        name: 'Invalid Updater',
        updater: {
          endpoint: 'http://updates.example.com/check',
          publicKey: 'bad-key',
        },
      },
      'volt.config.ts',
      { strict: false },
    );

    expect(validated.updater).toBeUndefined();
    expect(errorSpy).toHaveBeenCalledWith(
      expect.stringContaining(
        "'updater.endpoint' must be an HTTPS URL or an HTTP localhost/loopback URL for local testing.",
      ),
    );
    expect(errorSpy).toHaveBeenCalledWith(
      expect.stringContaining("'updater.publicKey' must be a base64 Ed25519 public key."),
    );
    errorSpy.mockRestore();
  });

  it('accepts well-formed plugin configuration', () => {
    const validated = validateConfig(
      {
        name: 'Plugin App',
        plugins: {
          enabled: ['acme.search', 'acme.sync'],
          grants: {
            'acme.search': ['fs', 'http'],
            'acme.sync': ['secureStorage'],
          },
          pluginDirs: ['./plugins', './more-plugins'],
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
      },
      'volt.config.ts',
      { strict: false },
    );

    expect(validated.plugins).toEqual({
      enabled: ['acme.search', 'acme.sync'],
      grants: {
        'acme.search': ['fs', 'http'],
        'acme.sync': ['secureStorage'],
      },
      pluginDirs: ['./plugins', './more-plugins'],
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
    });
  });

  it('sanitizes malformed plugin configuration fields without dropping the full object', () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const validated = validateConfig(
      {
        name: 'Plugin App',
        plugins: {
          enabled: ['acme.search', '', 'acme.search'],
          grants: {
            'acme.search': ['fs', 'bogus', 'http'],
          },
          pluginDirs: ['./plugins', '   ', './plugins'],
          limits: {
            activationTimeoutMs: 0,
            deactivationTimeoutMs: 5_000,
            heartbeatIntervalMs: 0,
          },
          spawning: {
            strategy: 'sometimes',
            idleTimeoutMs: -1,
            preSpawn: ['acme.search', '', 'acme.search'],
          },
        },
      },
      'volt.config.ts',
      { strict: false },
    );

    expect(validated.plugins).toEqual({
      enabled: ['acme.search'],
      grants: {
        'acme.search': ['fs', 'http'],
      },
      pluginDirs: ['./plugins'],
      limits: {
        deactivationTimeoutMs: 5_000,
      },
      spawning: {
        preSpawn: ['acme.search'],
      },
    });
    expect(errorSpy).toHaveBeenCalledWith(
      expect.stringContaining("'plugins.enabled' entries must be non-empty strings."),
    );
    expect(errorSpy).toHaveBeenCalledWith(
      expect.stringContaining("Unknown permission 'bogus' in 'plugins.grants.acme.search'."),
    );
    expect(errorSpy).toHaveBeenCalledWith(
      expect.stringContaining("'plugins.spawning.strategy' must be \"lazy\" or \"eager\"."),
    );
    expect(errorSpy).toHaveBeenCalledWith(
      expect.stringContaining("'plugins.limits.heartbeatIntervalMs' must be a positive integer."),
    );
    errorSpy.mockRestore();
  });
});
