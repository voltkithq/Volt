import { getAnalyticsProfile, runAnalyticsBenchmark } from '../../examples/analytics-studio/src/backend-logic.ts';
import { startSyncStorm } from '../../examples/sync-storm/src/backend-logic.ts';
import { runWorkflowBenchmark } from '../../examples/workflow-lab/src/workflow.ts';

interface AnalyticsStudioConfig {
  datasetSize: number;
  iterations: number;
  searchTerm: string;
  minScore: number;
  topN: number;
}

interface SyncStormConfig {
  workerCount: number;
  ticksPerWorker: number;
  intervalMs: number;
  burstSize: number;
}

interface WorkflowLabConfig {
  batchSize: number;
  passes: number;
}

interface BenchmarkProfile {
  analyticsStudio: AnalyticsStudioConfig;
  syncStorm: SyncStormConfig;
  workflowLab: WorkflowLabConfig;
}

interface BenchmarkProfileOverrides {
  analyticsStudio?: Partial<AnalyticsStudioConfig>;
  syncStorm?: Partial<SyncStormConfig>;
  workflowLab?: Partial<WorkflowLabConfig>;
}

interface AnalyticsStudioSummary {
  datasetSize: number;
  iterations: number;
  backendDurationMs: number;
  roundTripMs: number;
  peakMatches: number;
  payloadBytes: number;
}

interface SyncStormSummary {
  workerCount: number;
  ticksPerWorker: number;
  totalTickEvents: number;
  snapshotEvents: number;
  backendDurationMs: number;
  roundTripMs: number;
  averageDriftMs: number;
  maxDriftMs: number;
  queuePeak: number;
}

interface WorkflowLabSummary {
  batchSize: number;
  passes: number;
  pipelineLength: number;
  backendDurationMs: number;
  roundTripMs: number;
  payloadBytes: number;
}

interface HeadlessBenchmarkSummary {
  analyticsStudio: BenchmarkCase<AnalyticsStudioSummary>;
  syncStorm: BenchmarkCase<SyncStormSummary>;
  workflowLab: BenchmarkCase<WorkflowLabSummary>;
}

interface BenchmarkCase<T> {
  status: 'ok' | 'error';
  error: string | null;
  metrics: T | null;
}

const DEFAULT_BENCHMARK_PROFILE: BenchmarkProfile = {
  analyticsStudio: {
    datasetSize: 50_000,
    iterations: 8,
    searchTerm: 'risk',
    minScore: 61,
    topN: 24,
  },
  syncStorm: {
    workerCount: 20,
    ticksPerWorker: 96,
    intervalMs: 2,
    burstSize: 8,
  },
  workflowLab: {
    batchSize: 6_000,
    passes: 4,
  },
};

async function main(): Promise<void> {
  const profile = loadBenchmarkProfile();
  const summary: HeadlessBenchmarkSummary = {
    analyticsStudio: await captureCase(() => Promise.resolve(runAnalyticsStudioNodeBenchmark(profile.analyticsStudio))),
    syncStorm: await captureCase(() => runSyncStormNodeBenchmark(profile.syncStorm)),
    workflowLab: await captureCase(() => Promise.resolve(runWorkflowLabNodeBenchmark(profile.workflowLab))),
  };

  console.log(`VOLT_NODE_BENCH_JSON:${JSON.stringify(summary)}`);
}

async function captureCase<T>(runner: () => Promise<T>): Promise<BenchmarkCase<T>> {
  try {
    return {
      status: 'ok',
      error: null,
      metrics: await runner(),
    };
  } catch (error) {
    return {
      status: 'error',
      error: error instanceof Error ? error.message : String(error),
      metrics: null,
    };
  }
}

function runAnalyticsStudioNodeBenchmark(config: AnalyticsStudioConfig): AnalyticsStudioSummary {
  getAnalyticsProfile(config.datasetSize);

  const startedAt = Date.now();
  const result = runAnalyticsBenchmark(config);

  return {
    datasetSize: result.datasetSize,
    iterations: result.iterations,
    backendDurationMs: result.backendDurationMs,
    roundTripMs: Date.now() - startedAt,
    peakMatches: result.peakMatches,
    payloadBytes: result.payloadBytes,
  };
}

async function runSyncStormNodeBenchmark(config: SyncStormConfig): Promise<SyncStormSummary> {
  const startedAt = Date.now();
  const execution = startSyncStorm(
    config,
    {
      tick() {
        return;
      },
      snapshot() {
        return;
      },
      complete() {
        return;
      },
    },
  );

  const summary = await execution.done;
  return {
    workerCount: summary.workerCount,
    ticksPerWorker: summary.ticksPerWorker,
    totalTickEvents: summary.totalTickEvents,
    snapshotEvents: summary.snapshotEvents,
    backendDurationMs: summary.backendDurationMs,
    roundTripMs: Date.now() - startedAt,
    averageDriftMs: summary.averageDriftMs,
    maxDriftMs: summary.maxDriftMs,
    queuePeak: summary.queuePeak,
  };
}

function runWorkflowLabNodeBenchmark(config: WorkflowLabConfig): WorkflowLabSummary {
  const startedAt = Date.now();
  const result = runWorkflowBenchmark(config);

  return {
    batchSize: result.batchSize,
    passes: result.passes,
    pipelineLength: result.pipeline.length,
    backendDurationMs: result.backendDurationMs,
    roundTripMs: Date.now() - startedAt,
    payloadBytes: result.payloadBytes,
  };
}

function loadBenchmarkProfile(): BenchmarkProfile {
  const rawProfile = process.env.VOLT_BENCH_PROFILE_JSON;
  if (!rawProfile) {
    return {
      analyticsStudio: { ...DEFAULT_BENCHMARK_PROFILE.analyticsStudio },
      syncStorm: { ...DEFAULT_BENCHMARK_PROFILE.syncStorm },
      workflowLab: { ...DEFAULT_BENCHMARK_PROFILE.workflowLab },
    };
  }

  let overrides: BenchmarkProfileOverrides;
  try {
    overrides = JSON.parse(rawProfile) as BenchmarkProfileOverrides;
  } catch (error) {
    throw new Error(
      `[bench] Failed to parse VOLT_BENCH_PROFILE_JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
  }

  return {
    analyticsStudio: {
      ...DEFAULT_BENCHMARK_PROFILE.analyticsStudio,
      ...sanitizeAnalyticsStudioConfig(overrides.analyticsStudio),
    },
    syncStorm: {
      ...DEFAULT_BENCHMARK_PROFILE.syncStorm,
      ...sanitizeSyncStormConfig(overrides.syncStorm),
    },
    workflowLab: {
      ...DEFAULT_BENCHMARK_PROFILE.workflowLab,
      ...sanitizeWorkflowLabConfig(overrides.workflowLab),
    },
  };
}

function sanitizeAnalyticsStudioConfig(
  overrides: Partial<AnalyticsStudioConfig> | undefined,
): Partial<AnalyticsStudioConfig> {
  if (!overrides) {
    return {};
  }

  return {
    datasetSize: readPositiveInteger(overrides.datasetSize),
    iterations: readPositiveInteger(overrides.iterations),
    searchTerm: typeof overrides.searchTerm === 'string' && overrides.searchTerm.length > 0
      ? overrides.searchTerm
      : undefined,
    minScore: readFiniteNumber(overrides.minScore),
    topN: readPositiveInteger(overrides.topN),
  };
}

function sanitizeSyncStormConfig(
  overrides: Partial<SyncStormConfig> | undefined,
): Partial<SyncStormConfig> {
  if (!overrides) {
    return {};
  }

  return {
    workerCount: readPositiveInteger(overrides.workerCount),
    ticksPerWorker: readPositiveInteger(overrides.ticksPerWorker),
    intervalMs: readPositiveInteger(overrides.intervalMs),
    burstSize: readPositiveInteger(overrides.burstSize),
  };
}

function sanitizeWorkflowLabConfig(
  overrides: Partial<WorkflowLabConfig> | undefined,
): Partial<WorkflowLabConfig> {
  if (!overrides) {
    return {};
  }

  return {
    batchSize: readPositiveInteger(overrides.batchSize),
    passes: readPositiveInteger(overrides.passes),
  };
}

function readPositiveInteger(value: unknown): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return undefined;
  }
  const normalized = Math.round(value);
  return normalized > 0 ? normalized : undefined;
}

function readFiniteNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

void main();
