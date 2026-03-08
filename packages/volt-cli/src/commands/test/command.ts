import { fileURLToPath } from 'node:url';
import { loadVoltTestConfig, runSuites } from '@voltkit/volt-test';

export interface TestCommandOptions {
  config?: string;
  suite?: string | string[];
  list?: boolean;
  timeout?: string;
  retries?: string;
  artifactsDir?: string;
}

export async function testCommand(options: TestCommandOptions): Promise<void> {
  const cwd = process.cwd();
  const suiteNames = normalizeSuiteNames(options.suite);
  const timeoutMs = parseTimeoutMs(options.timeout);
  const retries = parseRetryCount(options.retries);
  const loadedConfig = await loadVoltTestConfig(cwd, {
    configPath: options.config,
    strict: true,
  });

  if (options.list) {
    console.log(`[volt:test] config: ${loadedConfig.configPath}`);
    for (const suite of loadedConfig.config.suites) {
      console.log(`- ${suite.name}`);
    }
    return;
  }

  await runSuites(loadedConfig.config, {
    cliEntryPath: resolveCliEntryPath(),
    suiteNames,
    timeoutMs,
    retries,
    artifactDir: options.artifactsDir,
    repoRoot: cwd,
    logger: {
      log: (message) => console.log(message),
      warn: (message) => console.warn(message),
      error: (message) => console.error(message),
    },
  });
}

function normalizeSuiteNames(raw: string | string[] | undefined): string[] | undefined {
  if (!raw) {
    return undefined;
  }
  const values = Array.isArray(raw) ? raw : [raw];
  const names = values
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return names.length > 0 ? names : undefined;
}

function parseTimeoutMs(raw: string | undefined): number | undefined {
  if (!raw) {
    return undefined;
  }
  const timeoutMs = Number(raw);
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error(`[volt:test] Invalid --timeout value "${raw}". Expected positive milliseconds.`);
  }
  return timeoutMs;
}

function parseRetryCount(raw: string | undefined): number | undefined {
  if (!raw) {
    return undefined;
  }

  const retries = Number(raw);
  if (!Number.isInteger(retries) || retries < 0) {
    throw new Error(`[volt:test] Invalid --retries value "${raw}". Expected a non-negative integer.`);
  }

  return retries;
}

function resolveCliEntryPath(): string {
  return fileURLToPath(new URL('../../index.js', import.meta.url));
}

export const __testOnly = {
  normalizeSuiteNames,
  parseRetryCount,
  parseTimeoutMs,
  resolveCliEntryPath,
};
