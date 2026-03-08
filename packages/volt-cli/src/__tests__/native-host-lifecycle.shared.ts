import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach } from 'vitest';

export const HOST_FIXTURE_PATH = fileURLToPath(new URL('./fixtures/fake-native-host.mjs', import.meta.url));
export const DEFAULT_STARTUP_TIMEOUT_MS = 3000;
export const DEFAULT_PID_FILE_TIMEOUT_MS = 3000;
export const DEFAULT_RUNTIME_SHUTDOWN_TIMEOUT_MS = 5000;

const trackedTempDirs: string[] = [];
const trackedPids = new Set<number>();

function createTempDir(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  trackedTempDirs.push(dir);
  return dir;
}

export function fixtureWindowConfig() {
  return {
    name: 'fixture-app',
    permissions: [],
    jsId: 'fixture-window-id',
    url: 'http://localhost:5173',
    devtools: true,
    window: {
      title: 'Fixture App',
      width: 800,
      height: 600,
      resizable: true,
      decorations: true,
    },
  };
}

export function createPidFilePath(): string {
  return join(createTempDir('volt-host-test-'), 'host.pid');
}

export function trackSpawnedPid(pid: number): void {
  trackedPids.add(pid);
}

function readPid(pidFilePath: string): number {
  const pid = Number.parseInt(readFileSync(pidFilePath, 'utf8').trim(), 10);
  if (!Number.isInteger(pid)) {
    throw new Error(`Invalid pid file contents at ${pidFilePath}`);
  }
  trackedPids.add(pid);
  return pid;
}

export async function waitForPidFile(pidFilePath: string, timeoutMs = DEFAULT_PID_FILE_TIMEOUT_MS): Promise<number> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(pidFilePath)) {
      try {
        return readPid(pidFilePath);
      } catch {
        // File can briefly exist with incomplete contents while being written.
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(`PID file was not written in time: ${pidFilePath}`);
}

function processExists(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    const code = (error as NodeJS.ErrnoException).code;
    return code === 'EPERM';
  }
}

export async function waitForProcessExit(pid: number, timeoutMs = 6000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!processExists(pid)) {
      return true;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  return !processExists(pid);
}

export async function withTimeout<T>(promise: Promise<T>, timeoutMs: number, label: string): Promise<T> {
  let timer: NodeJS.Timeout | null = null;
  const timeoutPromise = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} timed out after ${timeoutMs}ms`)), timeoutMs);
    timer.unref();
  });

  try {
    return await Promise.race([promise, timeoutPromise]);
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
}

afterEach(async () => {
  for (const pid of trackedPids) {
    if (processExists(pid)) {
      try {
        process.kill(pid);
      } catch {
        // Best-effort cleanup.
      }
      await waitForProcessExit(pid, 2000);
    }
  }
  trackedPids.clear();

  while (trackedTempDirs.length > 0) {
    const dir = trackedTempDirs.pop();
    if (!dir) {
      continue;
    }
    rmSync(dir, { recursive: true, force: true });
  }
});
