import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { defineTestConfig, loadVoltTestConfig, validateTestConfig } from './config.js';

describe('defineTestConfig', () => {
  it('returns the same config object', () => {
    const config = defineTestConfig({
      timeoutMs: 5_000,
      suites: [
        {
          name: 'suite-a',
          async run() {},
        },
      ],
    });

    expect(config.timeoutMs).toBe(5_000);
    expect(config.suites).toHaveLength(1);
  });
});

describe('validateTestConfig', () => {
  it('rejects duplicate suite names', () => {
    expect(() =>
      validateTestConfig({
        suites: [
          { name: 'dup', async run() {} },
          { name: 'dup', async run() {} },
        ],
      }),
    ).toThrow('Duplicate suite name');
  });

  it('rejects non-positive timeout values', () => {
    expect(() =>
      validateTestConfig({
        timeoutMs: 0,
        suites: [{ name: 'suite-a', async run() {} }],
      }),
    ).toThrow('expected positive milliseconds');
  });

  it('rejects negative retries', () => {
    expect(() =>
      validateTestConfig({
        retries: -1,
        suites: [{ name: 'suite-a', async run() {} }],
      }),
    ).toThrow('expected non-negative integer');
  });

  it('rejects empty artifactDir', () => {
    expect(() =>
      validateTestConfig({
        artifactDir: '   ',
        suites: [{ name: 'suite-a', async run() {} }],
      }),
    ).toThrow('artifactDir');
  });
});

describe('loadVoltTestConfig', () => {
  it('loads config from an explicit path', async () => {
    const root = mkdtempSync(join(tmpdir(), 'volt-test-config-'));
    const configPath = join(root, 'my-config.mjs');
    writeFileSync(
      configPath,
      [
        'export default {',
        '  timeoutMs: 1234,',
        '  suites: [{',
        "    name: 'suite-a',",
        '    async run() {}',
        '  }],',
        '};',
      ].join('\n'),
      'utf8',
    );

    const loaded = await loadVoltTestConfig(root, { configPath: 'my-config.mjs' });
    expect(loaded.configPath).toBe(configPath);
    expect(loaded.config.timeoutMs).toBe(1234);
    expect(loaded.config.suites).toHaveLength(1);
  });

  it('throws when no config file exists in strict mode', async () => {
    const root = mkdtempSync(join(tmpdir(), 'volt-test-config-missing-'));
    await expect(loadVoltTestConfig(root, { strict: true })).rejects.toThrow('No test config found');
  });
});
