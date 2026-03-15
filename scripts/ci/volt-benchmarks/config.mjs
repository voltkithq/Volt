import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const repoRoot = resolve(__dirname, '..', '..', '..');
const artifactDir = resolve(repoRoot, 'artifacts', 'volt-benchmarks', process.platform);
const bundleDir = mkdtempSync(join(tmpdir(), 'volt-bench-'));
const isReleaseMode = process.argv.includes('--release');
const isSweepMode = process.argv.includes('--sweep');
const summaryPath = resolve(artifactDir, getSummaryFileName());
const nodeBenchmarkScript = resolve(repoRoot, 'scripts', 'ci', 'volt-node-benchmark.ts');
const nodeBenchmarkBundlePath = resolve(bundleDir, 'node-benchmark.bundle.mjs');
const backendBuildModulePath = resolve(
  repoRoot,
  'packages',
  'volt-cli',
  'dist',
  'commands',
  'build',
  'backend.js',
);

const benchmarkProjects = {
  analytics: {
    projectDir: resolve(repoRoot, 'examples', 'analytics-studio'),
    backendEntry: 'src/headless-benchmark.ts',
    bundlePath: resolve(bundleDir, 'analytics-studio.backend.mjs'),
    envName: 'VOLT_BENCH_ANALYTICS_BUNDLE',
  },
  sync: {
    projectDir: resolve(repoRoot, 'examples', 'sync-storm'),
    backendEntry: 'src/headless-benchmark.ts',
    bundlePath: resolve(bundleDir, 'sync-storm.backend.mjs'),
    envName: 'VOLT_BENCH_SYNC_BUNDLE',
  },
  workflow: {
    projectDir: resolve(repoRoot, 'examples', 'workflow-lab'),
    backendEntry: 'src/headless-benchmark.ts',
    bundlePath: resolve(bundleDir, 'workflow-lab.backend.mjs'),
    envName: 'VOLT_BENCH_WORKFLOW_BUNDLE',
  },
};

const benchmarkSweepProfiles = [
  {
    id: 'small',
    config: {
      analyticsStudio: {
        datasetSize: 10_000,
        iterations: 4,
        searchTerm: 'risk',
        minScore: 61,
        topN: 18,
      },
      syncStorm: {
        workerCount: 8,
        ticksPerWorker: 48,
        intervalMs: 3,
        burstSize: 4,
      },
      workflowLab: {
        batchSize: 1_500,
        passes: 2,
      },
    },
  },
  {
    id: 'medium',
    config: {
      analyticsStudio: {
        datasetSize: 25_000,
        iterations: 6,
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
        batchSize: 3_000,
        passes: 3,
      },
    },
  },
  {
    id: 'large',
    config: {
      analyticsStudio: {
        datasetSize: 50_000,
        iterations: 8,
        searchTerm: 'risk',
        minScore: 61,
        topN: 24,
      },
      syncStorm: {
        workerCount: 40,
        ticksPerWorker: 160,
        intervalMs: 2,
        burstSize: 8,
      },
      workflowLab: {
        batchSize: 6_000,
        passes: 4,
      },
    },
  },
];

function getSummaryFileName() {
  if (isSweepMode && isReleaseMode) {
    return 'benchmark-summary.sweep.release.json';
  }
  if (isSweepMode) {
    return 'benchmark-summary.sweep.json';
  }
  if (isReleaseMode) {
    return 'benchmark-summary.release.json';
  }
  return 'benchmark-summary.json';
}

export {
  artifactDir,
  backendBuildModulePath,
  benchmarkProjects,
  benchmarkSweepProfiles,
  bundleDir,
  isReleaseMode,
  isSweepMode,
  nodeBenchmarkBundlePath,
  nodeBenchmarkScript,
  repoRoot,
  summaryPath,
};
