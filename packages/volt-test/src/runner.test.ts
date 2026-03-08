import { existsSync } from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { __testOnly, runSuites } from './runner.js';
import type { VoltTestLogger, VoltTestSuiteContext } from './types.js';

function createLogger(): VoltTestLogger {
  return {
    log: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  };
}

function createRunRoot(): string {
  return mkdtempSync(join(tmpdir(), 'volt-test-runner-'));
}

describe('runner helpers', () => {
  it('selects all suites when no filter is provided', () => {
    const suites = [
      { name: 'a', async run() {} },
      { name: 'b', async run() {} },
    ];
    expect(__testOnly.selectSuites(suites)).toHaveLength(2);
  });

  it('throws for unknown suites', () => {
    const suites = [{ name: 'a', async run() {} }];
    expect(() => __testOnly.selectSuites(suites, ['missing'])).toThrow('none of the requested suites');
  });
});

describe('runSuites', () => {
  it('runs selected suites with context values and artifacts', async () => {
    const logger = createLogger();
    const contexts: VoltTestSuiteContext[] = [];
    const root = createRunRoot();

    await runSuites(
      {
        timeoutMs: 5_000,
        suites: [
          {
            name: 'suite-a',
            async run(context) {
              contexts.push(context);
            },
          },
          {
            name: 'suite-b',
            async run(context) {
              contexts.push(context);
            },
          },
        ],
      },
      {
        cliEntryPath: './packages/volt-cli/dist/index.js',
        suiteNames: ['suite-b'],
        repoRoot: root,
        artifactDir: 'artifacts/test-output',
        logger,
      },
    );

    expect(contexts).toHaveLength(1);
    expect(contexts[0].timeoutMs).toBe(5_000);
    expect(contexts[0].suiteName).toBe('suite-b');
    expect(contexts[0].attempt).toBe(1);
    expect(contexts[0].artifactsDir).toContain(join('suite-b', 'attempt-1'));
    expect(existsSync(join(root, 'artifacts', 'test-output', 'run-summary.json'))).toBe(true);
  });

  it('retries failing suites and marks flaky pass', async () => {
    const logger = createLogger();
    const root = createRunRoot();
    let attempts = 0;

    await runSuites(
      {
        retries: 1,
        suites: [
          {
            name: 'sometimes-fails',
            async run() {
              attempts += 1;
              if (attempts === 1) {
                throw new Error('first attempt failure');
              }
            },
          },
        ],
      },
      {
        cliEntryPath: './packages/volt-cli/dist/index.js',
        repoRoot: root,
        artifactDir: 'artifacts/retry-output',
        captureScreenshots: false,
        logger,
      },
    );

    expect(attempts).toBe(2);
    expect(existsSync(join(root, 'artifacts', 'retry-output', 'flake-report.json'))).toBe(true);
  });

  it('enforces suite timeout', async () => {
    const logger = createLogger();
    const root = createRunRoot();

    await expect(
      runSuites(
        {
          suites: [
            {
              name: 'slow',
              async run() {
                await new Promise<void>(() => {
                  // never resolve
                });
              },
            },
          ],
        },
        {
          cliEntryPath: './packages/volt-cli/dist/index.js',
          timeoutMs: 30,
          repoRoot: root,
          artifactDir: 'artifacts/timeout-output',
          captureScreenshots: false,
          logger,
        },
      ),
    ).rejects.toThrow('timed out');
  });
});
