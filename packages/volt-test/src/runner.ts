import { join, resolve } from 'node:path';
import {
  captureDesktopScreenshot,
  createRunArtifactRoot,
  createSuiteAttemptArtifactDir,
  writeJsonArtifact,
} from './artifacts.js';
import { validateTestConfig } from './config.js';
import { sanitizePathSegment } from './path.js';
import type {
  RunSuitesOptions,
  VoltTestArtifactCaptureResult,
  VoltTestConfig,
  VoltTestLogger,
  VoltTestSuite,
} from './types.js';

const DEFAULT_TIMEOUT_MS = 120_000;
const DEFAULT_RETRIES = 0;

interface SuiteAttemptResult {
  attempt: number;
  durationMs: number;
  status: 'passed' | 'failed';
  error?: string;
  screenshotPath?: string;
}

interface SuiteRunSummary {
  name: string;
  attempts: SuiteAttemptResult[];
  flaky: boolean;
  durationMs: number;
  passed: boolean;
}

interface SuiteRunResult {
  summary: SuiteRunSummary;
  error: unknown;
}

interface RunSummary {
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  suites: SuiteRunSummary[];
}

export async function runSuites(config: VoltTestConfig, options: RunSuitesOptions): Promise<void> {
  validateTestConfig(config, 'test config');

  const logger = options.logger ?? toLogger(console);
  const selectedSuites = selectSuites(config.suites, options.suiteNames);
  const defaultTimeoutMs = options.timeoutMs ?? config.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const retries = options.retries ?? config.retries ?? DEFAULT_RETRIES;
  const captureScreenshots = options.captureScreenshots ?? true;
  const repoRoot = resolve(options.repoRoot ?? process.cwd());
  const cliEntryPath = resolve(options.cliEntryPath);
  const runArtifactRoot = createRunArtifactRoot(repoRoot, options.artifactDir ?? config.artifactDir);

  validateRetryCount(retries);

  logger.log(`[volt:test] running ${selectedSuites.length} suite(s)`);
  logger.log(`[volt:test] retries per suite: ${retries}`);
  logger.log(`[volt:test] artifact root: ${runArtifactRoot}`);

  const startedAt = Date.now();
  const suiteSummaries: SuiteRunSummary[] = [];
  let completed = 0;
  let firstFailure: unknown = null;

  for (const suite of selectedSuites) {
    const suiteTimeoutMs = suite.timeoutMs ?? defaultTimeoutMs;
    const suiteStartedAt = Date.now();
    const suiteResult = await runSuiteWithRetries({
      suite,
      retries,
      suiteTimeoutMs,
      repoRoot,
      cliEntryPath,
      runArtifactRoot,
      captureScreenshots,
      logger,
    });
    const suiteSummary = suiteResult.summary;

    suiteSummaries.push(suiteSummary);
    completed += 1;
    if (suiteSummary.passed) {
      logger.log(`[volt:test] [${suite.name}] passed in ${suiteSummary.durationMs}ms`);
    } else {
      logger.error(`[volt:test] [${suite.name}] failed in ${suiteSummary.durationMs}ms`);
    }
    if (suiteSummary.passed && suiteSummary.flaky) {
      logger.warn(`[volt:test] [${suite.name}] marked flaky: passed after retry.`);
    }

    writeJsonArtifact(join(runArtifactRoot, sanitizePathSegment(suite.name), 'suite-summary.json'), {
      ...suiteSummary,
      finishedAt: new Date().toISOString(),
      suiteTimeoutMs,
      suiteDurationMs: Date.now() - suiteStartedAt,
    });

    if (suiteResult.error && !firstFailure) {
      firstFailure = suiteResult.error;
      break;
    }
  }

  const finishedAt = Date.now();
  const runSummary: RunSummary = {
    startedAt: new Date(startedAt).toISOString(),
    finishedAt: new Date(finishedAt).toISOString(),
    durationMs: finishedAt - startedAt,
    suites: suiteSummaries,
  };

  writeJsonArtifact(join(runArtifactRoot, 'run-summary.json'), runSummary);

  const flakySuites = suiteSummaries.filter((suite) => suite.flaky).map((suite) => suite.name);
  if (flakySuites.length > 0) {
    writeJsonArtifact(join(runArtifactRoot, 'flake-report.json'), {
      flakySuites,
      count: flakySuites.length,
      generatedAt: new Date().toISOString(),
    });
  }

  logger.log(
    `[volt:test] completed ${completed}/${selectedSuites.length} suite(s) in ${finishedAt - startedAt}ms`,
  );

  if (firstFailure) {
    throw firstFailure;
  }
}

async function runSuiteWithRetries(args: {
  suite: VoltTestSuite;
  retries: number;
  suiteTimeoutMs: number;
  repoRoot: string;
  cliEntryPath: string;
  runArtifactRoot: string;
  captureScreenshots: boolean;
  logger: VoltTestLogger;
}): Promise<SuiteRunResult> {
  const { suite, retries, suiteTimeoutMs, repoRoot, cliEntryPath, runArtifactRoot, captureScreenshots, logger } = args;
  const attempts: SuiteAttemptResult[] = [];
  const maxAttempts = retries + 1;
  const suiteStart = Date.now();

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    const attemptStart = Date.now();
    const attemptArtifactDir = createSuiteAttemptArtifactDir(runArtifactRoot, suite.name, attempt);
    const prefixedLogger = withPrefix(logger, `[volt:test] [${suite.name}] [attempt ${attempt}/${maxAttempts}]`);
    prefixedLogger.log('start');
    prefixedLogger.log(`artifacts: ${attemptArtifactDir}`);

    const captureScreenshot = async (name = 'screenshot'): Promise<VoltTestArtifactCaptureResult> => {
      const screenshotPath = join(attemptArtifactDir, `${sanitizePathSegment(name)}.png`);
      if (!captureScreenshots) {
        return { path: screenshotPath, captured: false };
      }
      const captured = await captureDesktopScreenshot(screenshotPath, prefixedLogger);
      return { path: screenshotPath, captured };
    };

    try {
      await withTimeout(
        suite.run({
          repoRoot,
          cliEntryPath,
          logger: prefixedLogger,
          timeoutMs: suiteTimeoutMs,
          suiteName: suite.name,
          attempt,
          artifactsDir: attemptArtifactDir,
          captureScreenshot,
        }),
        suiteTimeoutMs,
        suite.name,
      );

      attempts.push({
        attempt,
        durationMs: Date.now() - attemptStart,
        status: 'passed',
      });

      return {
        summary: {
          name: suite.name,
          attempts,
          flaky: attempt > 1,
          durationMs: Date.now() - suiteStart,
          passed: true,
        },
        error: null,
      };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      let screenshotPath: string | undefined;
      if (captureScreenshots) {
        const screenshot = await captureScreenshot(`failure-${attempt}`);
        if (screenshot.captured) {
          screenshotPath = screenshot.path;
        }
      }

      attempts.push({
        attempt,
        durationMs: Date.now() - attemptStart,
        status: 'failed',
        error: message,
        screenshotPath,
      });

      prefixedLogger.error(message);
      if (attempt < maxAttempts) {
        prefixedLogger.warn('retrying after failure');
        continue;
      }
      return {
        summary: {
          name: suite.name,
          attempts,
          flaky: false,
          durationMs: Date.now() - suiteStart,
          passed: false,
        },
        error,
      };
    }
  }

  const internalError = new Error('[volt:test] internal runner error: no suite attempts executed.');
  return {
    summary: {
      name: suite.name,
      attempts: [],
      flaky: false,
      durationMs: 0,
      passed: false,
    },
    error: internalError,
  };
}

function selectSuites(suites: readonly VoltTestSuite[], names?: readonly string[]): VoltTestSuite[] {
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

async function withTimeout(promise: Promise<void>, timeoutMs: number, suiteName: string): Promise<void> {
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

function withPrefix(logger: VoltTestLogger, prefix: string): VoltTestLogger {
  return {
    log: (message) => logger.log(`${prefix} ${message}`),
    warn: (message) => logger.warn(`${prefix} ${message}`),
    error: (message) => logger.error(`${prefix} ${message}`),
  };
}

function validateRetryCount(retries: number): void {
  if (!Number.isInteger(retries) || retries < 0) {
    throw new Error('[volt:test] retries must be a non-negative integer.');
  }
}

function toLogger(source: typeof console): VoltTestLogger {
  return {
    log: (message) => source.log(message),
    warn: (message) => source.warn(message),
    error: (message) => source.error(message),
  };
}

export const __testOnly = {
  selectSuites,
  withTimeout,
  withPrefix,
};
