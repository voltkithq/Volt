import type { VoltTestLogger, VoltTestSuite } from '../types.js';

export const DEFAULT_TIMEOUT_MS = 120_000;
export const DEFAULT_RETRIES = 0;

export function selectSuites(
  suites: readonly VoltTestSuite[],
  names?: readonly string[],
): VoltTestSuite[] {
  if (!names || names.length === 0) {
    return [...suites];
  }

  const wanted = new Set(names);
  const selected = suites.filter((suite) => wanted.has(suite.name));
  if (selected.length === 0) {
    throw new Error(`[volt:test] none of the requested suites were found: ${names.join(', ')}`);
  }

  const missing = names.filter((name) => !selected.some((suite) => suite.name === name));
  if (missing.length > 0) {
    throw new Error(`[volt:test] unknown suite(s): ${missing.join(', ')}`);
  }

  return selected;
}

export async function withTimeout(
  promise: Promise<void>,
  timeoutMs: number,
  suiteName: string,
): Promise<void> {
  let timeoutHandle: NodeJS.Timeout | null = null;
  try {
    await Promise.race([
      promise,
      new Promise<void>((_, reject) => {
        timeoutHandle = setTimeout(() => {
          reject(new Error(`[volt:test] suite "${suiteName}" timed out after ${timeoutMs}ms`));
        }, timeoutMs);
      }),
    ]);
  } finally {
    if (timeoutHandle) {
      clearTimeout(timeoutHandle);
    }
  }
}

export function withPrefix(logger: VoltTestLogger, prefix: string): VoltTestLogger {
  return {
    log: (message) => logger.log(`${prefix} ${message}`),
    warn: (message) => logger.warn(`${prefix} ${message}`),
    error: (message) => logger.error(`${prefix} ${message}`),
  };
}

export function validateRetryCount(retries: number): void {
  if (!Number.isInteger(retries) || retries < 0) {
    throw new Error('[volt:test] retries must be a non-negative integer.');
  }
}

export function toLogger(source: typeof console): VoltTestLogger {
  return {
    log: (message) => source.log(message),
    warn: (message) => source.warn(message),
    error: (message) => source.error(message),
  };
}
