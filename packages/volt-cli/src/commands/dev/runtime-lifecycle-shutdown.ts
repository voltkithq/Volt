import type { ChildProcess } from 'node:child_process';

export async function waitForChildExit(
  exited: Promise<void>,
  timeoutMs: number,
): Promise<void> {
  await Promise.race([
    exited.catch(() => {}),
    new Promise<void>((resolve) => {
      const timer = setTimeout(resolve, timeoutMs);
      timer.unref();
    }),
  ]);
}

export async function killChildAndWait(
  child: ChildProcess,
  exited: Promise<void>,
  timeoutMs: number,
): Promise<void> {
  if (!child.killed) {
    child.kill();
  }
  await waitForChildExit(exited, timeoutMs);
}

export function scheduleForcedChildKill(
  child: ChildProcess,
  delayMs: number,
): void {
  setTimeout(() => {
    if (!child.killed) {
      child.kill();
    }
  }, delayMs).unref();
}
