export interface SyncStormOptions {
  workerCount?: number;
  ticksPerWorker?: number;
  intervalMs?: number;
  burstSize?: number;
}

export interface SyncTickPayload {
  scenarioId: string;
  workerId: number;
  tickIndex: number;
  issuedAt: number;
  driftMs: number;
  queueDepth: number;
  topic: string;
  bytes: number;
}

export interface SyncSnapshotPayload {
  scenarioId: string;
  issuedAt: number;
  completedWorkers: number;
  activeWorkers: number;
  totalTickEvents: number;
  queuePeak: number;
}

export interface SyncStormSummary {
  scenarioId: string;
  workerCount: number;
  ticksPerWorker: number;
  intervalMs: number;
  burstSize: number;
  totalTickEvents: number;
  snapshotEvents: number;
  backendDurationMs: number;
  averageDriftMs: number;
  maxDriftMs: number;
  queuePeak: number;
  topicTotals: Record<string, number>;
}

export interface SyncStormExecution {
  scenarioId: string;
  config: Required<SyncStormOptions>;
  done: Promise<SyncStormSummary>;
}

const topics = ['replication', 'cache', 'presence', 'audit', 'merge', 'rebalance'];

function toRangeInteger(value: unknown, fallback: number, min: number, max: number): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, Math.round(numeric)));
}

function normalizeOptions(options: SyncStormOptions = {}): Required<SyncStormOptions> {
  return {
    workerCount: toRangeInteger(options.workerCount, 10, 1, 32),
    ticksPerWorker: toRangeInteger(options.ticksPerWorker, 36, 4, 160),
    intervalMs: toRangeInteger(options.intervalMs, 6, 1, 50),
    burstSize: toRangeInteger(options.burstSize, 6, 2, 24),
  };
}

function buildPayload(workerId: number, tickIndex: number, burstSize: number): string {
  const parts: string[] = [];
  for (let index = 0; index < burstSize; index += 1) {
    parts.push(`${topics[(workerId + tickIndex + index) % topics.length]}:${workerId}:${tickIndex}:${index}`);
  }
  return parts.join('|');
}

export function buildSyncStormPreset(): Required<SyncStormOptions> {
  return normalizeOptions();
}

export function startSyncStorm(
  options: SyncStormOptions,
  emit: {
    tick(payload: SyncTickPayload): void;
    snapshot(payload: SyncSnapshotPayload): void;
    complete(payload: SyncStormSummary): void;
  },
): SyncStormExecution {
  const config = normalizeOptions(options);
  const scenarioId = `sync-${Date.now()}-${Math.floor(Math.random() * 10_000)}`;

  const done = new Promise<SyncStormSummary>((resolve) => {
    const startedAt = Date.now();
    let totalTickEvents = 0;
    let snapshotEvents = 0;
    let totalDriftMs = 0;
    let maxDriftMs = 0;
    let queuePeak = 0;
    let completedWorkers = 0;
    const topicTotals: Record<string, number> = {};

    const emitSnapshot = (): void => {
      snapshotEvents += 1;
      emit.snapshot({
        scenarioId,
        issuedAt: Date.now(),
        completedWorkers,
        activeWorkers: config.workerCount - completedWorkers,
        totalTickEvents,
        queuePeak,
      });
    };

    for (let workerId = 0; workerId < config.workerCount; workerId += 1) {
      const workerStartedAt = Date.now();

      const scheduleTick = (tickIndex: number): void => {
        if (tickIndex >= config.ticksPerWorker) {
          completedWorkers += 1;
          emitSnapshot();
          if (completedWorkers === config.workerCount) {
            const summary: SyncStormSummary = {
              scenarioId,
              workerCount: config.workerCount,
              ticksPerWorker: config.ticksPerWorker,
              intervalMs: config.intervalMs,
              burstSize: config.burstSize,
              totalTickEvents,
              snapshotEvents,
              backendDurationMs: Date.now() - startedAt,
              averageDriftMs: totalTickEvents > 0 ? Number((totalDriftMs / totalTickEvents).toFixed(2)) : 0,
              maxDriftMs,
              queuePeak,
              topicTotals,
            };
            emit.complete(summary);
            resolve(summary);
          }
          return;
        }

        const expectedAt = workerStartedAt + (tickIndex + 1) * config.intervalMs;
        setTimeout(() => {
          const issuedAt = Date.now();
          const driftMs = Math.max(0, issuedAt - expectedAt);
          const queueDepth = ((workerId + 1) * 11 + tickIndex * 7) % 29 + (tickIndex % config.burstSize);
          const topic = topics[(workerId + tickIndex) % topics.length];
          const payload = buildPayload(workerId, tickIndex, config.burstSize);

          totalTickEvents += 1;
          totalDriftMs += driftMs;
          maxDriftMs = Math.max(maxDriftMs, driftMs);
          queuePeak = Math.max(queuePeak, queueDepth);
          topicTotals[topic] = (topicTotals[topic] ?? 0) + 1;

          emit.tick({
            scenarioId,
            workerId,
            tickIndex,
            issuedAt,
            driftMs,
            queueDepth,
            topic,
            bytes: payload.length,
          });

          if ((tickIndex + 1) % config.burstSize === 0 || tickIndex === config.ticksPerWorker - 1) {
            emitSnapshot();
          }

          scheduleTick(tickIndex + 1);
        }, config.intervalMs);
      };

      scheduleTick(0);
    }
  });

  return {
    scenarioId,
    config,
    done,
  };
}
