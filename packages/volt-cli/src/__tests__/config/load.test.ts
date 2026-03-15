import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { existsSync } from 'node:fs';

import { loadConfig } from '../../utils/config.js';

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
    expect(consoleWarn).toHaveBeenCalledWith(expect.stringContaining('No config file found'));
  });

  it('throws in strict mode when no config file exists', async () => {
    vi.mocked(existsSync).mockReturnValue(false);
    await expect(
      loadConfig('/fake/project', { strict: true, commandName: 'build' }),
    ).rejects.toThrow('No config file found');
  });

  it('throws when a config file exists but fails to load', async () => {
    vi.mocked(existsSync).mockReturnValue(true);
    await expect(loadConfig('/fake/project')).rejects.toThrow(/Failed to load.*config/);
  });
});
