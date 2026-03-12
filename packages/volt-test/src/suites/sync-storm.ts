import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { VoltAppLauncher } from '../launcher.js';
import type { VoltTestSuite } from '../types.js';

const RESULT_FILE = '.volt-benchmark-result.json';

export interface SyncStormBenchmarkSuiteOptions {
  name?: string;
  projectDir?: string;
  timeoutMs?: number;
}

export interface SyncStormBenchmarkPayload {
  ok: boolean;
  start: {
    scenarioId: string;
  };
  summary: {
    totalTickEvents: number;
    snapshotEvents: number;
    backendDurationMs: number;
    averageDriftMs: number;
    maxDriftMs: number;
  };
  renderer: {
    tickCount: number;
    snapshotCount: number;
    averageLagMs: number;
    maxLagMs: number;
  };
}

export function createSyncStormBenchmarkSuite(
  options: SyncStormBenchmarkSuiteOptions = {},
): VoltTestSuite {
  const name = options.name ?? 'sync-storm-benchmark';
  const projectDir = options.projectDir ?? 'examples/sync-storm';
  const timeoutMs = options.timeoutMs ?? 900_000;

  return {
    name,
    timeoutMs,
    async run(context) {
      const launcher = new VoltAppLauncher({
        repoRoot: context.repoRoot,
        cliEntryPath: context.cliEntryPath,
        logger: context.logger,
      });

      await launcher.run<SyncStormBenchmarkPayload>({
        sourceProjectDir: projectDir,
        resultFile: RESULT_FILE,
        timeoutMs: context.timeoutMs,
        prepareProject(projectDirPath) {
          writeFileSync(join(projectDirPath, 'src', 'main.ts'), SYNC_STORM_AUTORUN_SOURCE, 'utf8');
        },
        validatePayload: validateSyncStormPayload,
        artifactsDir: context.artifactsDir,
      });
    },
  };
}

function validateSyncStormPayload(payload: unknown): SyncStormBenchmarkPayload {
  const value = asRecord(payload);
  if (!value || value.ok !== true) {
    throw new Error(`[volt:test] sync-storm benchmark failed: ${JSON.stringify(payload)}`);
  }

  const start = asRecord(value.start);
  const summary = asRecord(value.summary);
  const renderer = asRecord(value.renderer);
  if (!start || !summary || !renderer || typeof start.scenarioId !== 'string') {
    throw new Error('[volt:test] sync-storm payload missing start, summary, or renderer metrics.');
  }

  return {
    ok: true,
    start: {
      scenarioId: start.scenarioId,
    },
    summary: {
      totalTickEvents: Number(summary.totalTickEvents),
      snapshotEvents: Number(summary.snapshotEvents),
      backendDurationMs: Number(summary.backendDurationMs),
      averageDriftMs: Number(summary.averageDriftMs),
      maxDriftMs: Number(summary.maxDriftMs),
    },
    renderer: {
      tickCount: Number(renderer.tickCount),
      snapshotCount: Number(renderer.snapshotCount),
      averageLagMs: Number(renderer.averageLagMs),
      maxLagMs: Number(renderer.maxLagMs),
    },
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

const SYNC_STORM_AUTORUN_SOURCE = `
interface VoltBridge {
  invoke<T = unknown>(method: string, args?: unknown): Promise<T>;
  on(event: string, callback: (payload: unknown) => void): void;
  off?(event: string, callback: (payload: unknown) => void): void;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

async function run(): Promise<void> {
  const bridge = window.__volt__;
  if (!bridge?.invoke || !bridge.on) {
    throw new Error('window.__volt__ bridge is unavailable');
  }

  try {
    const payload = await new Promise<unknown>((resolve, reject) => {
      let start: { scenarioId: string } | null = null;
      let tickCount = 0;
      let snapshotCount = 0;
      let totalLagMs = 0;
      let maxLagMs = 0;
      const timeout = setTimeout(() => {
        reject(new Error('timed out waiting for sync:complete'));
      }, 45000);

      const onTick = (eventPayload: unknown) => {
        const value = eventPayload as { scenarioId?: string; issuedAt?: number } | null;
        if (!value || value.scenarioId !== start?.scenarioId) {
          return;
        }
        tickCount += 1;
        const lagMs = Math.max(0, Date.now() - Number(value.issuedAt ?? Date.now()));
        totalLagMs += lagMs;
        maxLagMs = Math.max(maxLagMs, lagMs);
      };

      const onSnapshot = (eventPayload: unknown) => {
        const value = eventPayload as { scenarioId?: string } | null;
        if (!value || value.scenarioId !== start?.scenarioId) {
          return;
        }
        snapshotCount += 1;
      };

      const onComplete = (eventPayload: unknown) => {
        const value = eventPayload as { scenarioId?: string } | null;
        if (!value || value.scenarioId !== start?.scenarioId || start === null) {
          return;
        }
        clearTimeout(timeout);
        bridge.off?.('sync:tick', onTick);
        bridge.off?.('sync:snapshot', onSnapshot);
        bridge.off?.('sync:complete', onComplete);
        resolve({
          ok: true,
          start,
          summary: eventPayload,
          renderer: {
            tickCount,
            snapshotCount,
            averageLagMs: tickCount > 0 ? Math.round(totalLagMs / tickCount) : 0,
            maxLagMs,
          },
        });
      };

      bridge.on('sync:tick', onTick);
      bridge.on('sync:snapshot', onSnapshot);
      bridge.on('sync:complete', onComplete);

      void bridge.invoke<{ scenarioId: string }>('sync:run', {
        workerCount: 12,
        ticksPerWorker: 48,
        intervalMs: 4,
        burstSize: 8,
      }).then((value) => {
        start = value;
      }, reject);
    });

    await bridge.invoke('benchmark:complete', payload);
  } catch (error) {
    await bridge.invoke('benchmark:complete', {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

void run();
`.trimStart();
