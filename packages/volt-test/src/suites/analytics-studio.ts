import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { VoltAppLauncher } from '../launcher.js';
import type { VoltTestSuite } from '../types.js';

const RESULT_FILE = '.volt-benchmark-result.json';

export interface AnalyticsStudioBenchmarkSuiteOptions {
  name?: string;
  projectDir?: string;
  timeoutMs?: number;
}

export interface AnalyticsStudioBenchmarkPayload {
  ok: boolean;
  profile: {
    datasetSize: number;
    cachedSizes: number[];
  };
  benchmark: {
    datasetSize: number;
    iterations: number;
    backendDurationMs: number;
    peakMatches: number;
    payloadBytes: number;
  };
  roundTripMs: number;
}

export function createAnalyticsStudioBenchmarkSuite(
  options: AnalyticsStudioBenchmarkSuiteOptions = {},
): VoltTestSuite {
  const name = options.name ?? 'analytics-studio-benchmark';
  const projectDir = options.projectDir ?? 'examples/analytics-studio';
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

      await launcher.run<AnalyticsStudioBenchmarkPayload>({
        sourceProjectDir: projectDir,
        resultFile: RESULT_FILE,
        timeoutMs: context.timeoutMs,
        prepareProject(projectDirPath) {
          writeFileSync(join(projectDirPath, 'src', 'main.ts'), ANALYTICS_AUTORUN_SOURCE, 'utf8');
        },
        validatePayload: validateAnalyticsPayload,
        artifactsDir: context.artifactsDir,
      });
    },
  };
}

function validateAnalyticsPayload(payload: unknown): AnalyticsStudioBenchmarkPayload {
  const value = asRecord(payload);
  if (!value || value.ok !== true) {
    throw new Error(`[volt:test] analytics benchmark failed: ${JSON.stringify(payload)}`);
  }

  const profile = asRecord(value.profile);
  const benchmark = asRecord(value.benchmark);
  const roundTripMs = value.roundTripMs;
  if (!profile || !benchmark || typeof roundTripMs !== 'number') {
    throw new Error('[volt:test] analytics benchmark payload missing profile, benchmark, or roundTripMs.');
  }

  const datasetSize = profile.datasetSize;
  const cachedSizes = profile.cachedSizes;
  const iterations = benchmark.iterations;
  const backendDurationMs = benchmark.backendDurationMs;
  const peakMatches = benchmark.peakMatches;
  const payloadBytes = benchmark.payloadBytes;
  if (
    typeof datasetSize !== 'number'
    || !Array.isArray(cachedSizes)
    || typeof iterations !== 'number'
    || typeof backendDurationMs !== 'number'
    || typeof peakMatches !== 'number'
    || typeof payloadBytes !== 'number'
  ) {
    throw new Error('[volt:test] analytics benchmark payload has invalid numeric fields.');
  }

  return {
    ok: true,
    profile: {
      datasetSize,
      cachedSizes: cachedSizes.filter((entry): entry is number => typeof entry === 'number'),
    },
    benchmark: {
      datasetSize: Number(benchmark.datasetSize),
      iterations,
      backendDurationMs,
      peakMatches,
      payloadBytes,
    },
    roundTripMs,
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

const ANALYTICS_AUTORUN_SOURCE = `
interface VoltBridge {
  invoke<T = unknown>(method: string, args?: unknown): Promise<T>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

async function run(): Promise<void> {
  const bridge = window.__volt__;
  if (!bridge?.invoke) {
    throw new Error('window.__volt__.invoke is unavailable');
  }

  const startedAt = Date.now();
  try {
    const profile = await bridge.invoke('analytics:profile', { datasetSize: 50000 });
    const benchmark = await bridge.invoke('analytics:run', {
      datasetSize: 50000,
      iterations: 8,
      searchTerm: 'risk',
      minScore: 61,
      topN: 24,
    });
    await bridge.invoke('benchmark:complete', {
      ok: true,
      profile,
      benchmark,
      roundTripMs: Date.now() - startedAt,
    });
  } catch (error) {
    await bridge.invoke('benchmark:complete', {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

void run();
`.trimStart();
