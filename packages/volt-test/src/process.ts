import { readFileSync } from 'node:fs';
import type { ChildProcess } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import type { VoltTestLogger } from './types.js';

export interface ChildExitResult {
  code: number | null;
  signal: NodeJS.Signals | null;
}

export async function waitForFile(filePath: string, timeoutMs: number): Promise<boolean> {
  const startedAt = Date.now();
  while (Date.now() - startedAt <= timeoutMs) {
    try {
      readFileSync(filePath, 'utf8');
      return true;
    } catch {
      await delay(250);
    }
  }
  return false;
}

export async function waitForChildExit(
  child: ChildProcess,
  timeoutMs: number,
): Promise<ChildExitResult | null> {
  if (child.exitCode !== null || child.signalCode !== null) {
    return {
      code: child.exitCode,
      signal: child.signalCode,
    };
  }

  const exitPromise = new Promise<ChildExitResult>((resolve) => {
    child.once('exit', (code, signal) => {
      resolve({ code, signal });
    });
  });
  const timeoutPromise = delay(timeoutMs).then(() => null);

  return Promise.race([exitPromise, timeoutPromise]);
}

export async function terminateChildProcess(
  child: ChildProcess,
  reason: string,
  logger: VoltTestLogger,
): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) {
    return;
  }

  try {
    child.kill('SIGTERM');
  } catch (error) {
    logger.warn(
      `[volt:test] failed to send SIGTERM (${reason}): ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }

  const gracefulExit = await waitForChildExit(child, 5_000);
  if (gracefulExit) {
    return;
  }

  try {
    child.kill('SIGKILL');
  } catch (error) {
    throw new Error(
      `[volt:test] failed to send SIGKILL (${reason}): ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }

  const forcedExit = await waitForChildExit(child, 5_000);
  if (!forcedExit) {
    throw new Error(`[volt:test] child process did not exit after SIGKILL (${reason})`);
  }
}

export async function readJsonFileWithRetry<T>(filePath: string, timeoutMs: number): Promise<T> {
  const startedAt = Date.now();
  let lastError: unknown;
  while (Date.now() - startedAt <= timeoutMs) {
    try {
      const content = readFileSync(filePath, 'utf8');
      return JSON.parse(content) as T;
    } catch (error) {
      lastError = error;
      await delay(100);
    }
  }

  throw new Error(
    `[volt:test] failed to parse JSON at ${filePath} within ${timeoutMs}ms: ${
      lastError instanceof Error ? lastError.message : String(lastError)
    }`,
  );
}
