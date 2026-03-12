import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const repoRoot = resolve(__dirname, '..', '..');
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

function assertFile(filePath, description) {
  if (!existsSync(filePath)) {
    throw new Error(`[bench] Missing ${description}: ${filePath}`);
  }
}

function parsePrefixedJson(output, prefix) {
  const line = output
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .reverse()
    .find((entry) => entry.startsWith(prefix));

  if (!line) {
    throw new Error(`[bench] Missing ${prefix} marker in command output.`);
  }

  return JSON.parse(line.slice(prefix.length));
}

function ratio(boaValue, nodeValue) {
  if (typeof boaValue !== 'number' || typeof nodeValue !== 'number' || nodeValue <= 0) {
    return null;
  }
  return Number((boaValue / nodeValue).toFixed(2));
}

function metric(caseSummary, key) {
  return caseSummary?.metrics?.[key];
}

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

async function bundleExampleBackends() {
  assertFile(backendBuildModulePath, 'volt-cli backend build output');
  assertFile(nodeBenchmarkScript, 'node benchmark source');
  mkdirSync(bundleDir, { recursive: true });

  const backendBuildModule = await import(pathToFileURL(backendBuildModulePath).href);
  const { buildBackendBundle } = backendBuildModule;
  if (typeof buildBackendBundle !== 'function') {
    throw new Error('[bench] buildBackendBundle export is unavailable.');
  }

  const bundleEnv = {};
  for (const project of Object.values(benchmarkProjects)) {
    const backendEntryPath = resolve(project.projectDir, project.backendEntry);
    await buildBackendBundle(project.projectDir, backendEntryPath, project.bundlePath);
    bundleEnv[project.envName] = project.bundlePath;
  }

  await buildBackendBundle(repoRoot, nodeBenchmarkScript, nodeBenchmarkBundlePath);

  return bundleEnv;
}

function runNodeBaseline(extraEnv = {}) {
  assertFile(nodeBenchmarkBundlePath, 'node benchmark bundle');
  const output = execFileSync('node', [nodeBenchmarkBundlePath], {
    cwd: repoRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...extraEnv,
    },
    maxBuffer: 20 * 1024 * 1024,
  });
  return parsePrefixedJson(output, 'VOLT_NODE_BENCH_JSON:');
}

function runBoaBaseline(bundleEnv, extraEnv = {}) {
  const cargoArgs = [
    'test',
    '-p',
    'volt-runner',
    'headless_example_backends_emit_benchmark_summary',
  ];
  if (isReleaseMode) {
    cargoArgs.push('--release');
  }
  cargoArgs.push('--', '--ignored', '--nocapture');

  const output = execFileSync(
    'cargo',
    cargoArgs,
    {
      cwd: repoRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        ...bundleEnv,
        ...extraEnv,
      },
      maxBuffer: 20 * 1024 * 1024,
    },
  );
  return parsePrefixedJson(output, 'VOLT_BENCH_JSON:');
}

function buildRatios(nodeSummary, boaSummary) {
  return {
    analyticsStudio: {
      backendDurationMs: ratio(
        metric(boaSummary.analyticsStudio, 'backendDurationMs'),
        metric(nodeSummary.analyticsStudio, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(boaSummary.analyticsStudio, 'roundTripMs'),
        metric(nodeSummary.analyticsStudio, 'roundTripMs'),
      ),
    },
    syncStorm: {
      backendDurationMs: ratio(
        metric(boaSummary.syncStorm, 'backendDurationMs'),
        metric(nodeSummary.syncStorm, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(boaSummary.syncStorm, 'roundTripMs'),
        metric(nodeSummary.syncStorm, 'roundTripMs'),
      ),
    },
    workflowLab: {
      backendDurationMs: ratio(
        metric(boaSummary.workflowLab, 'backendDurationMs'),
        metric(nodeSummary.workflowLab, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(boaSummary.workflowLab, 'roundTripMs'),
        metric(nodeSummary.workflowLab, 'roundTripMs'),
      ),
    },
  };
}

function buildSingleSummary(nodeSummary, boaSummary) {
  return {
    generatedAt: new Date().toISOString(),
    platform: process.platform,
    mode: 'headless-backend-runtime',
    boaProfile: isReleaseMode ? 'release' : 'test',
    node: nodeSummary,
    boa: boaSummary,
    ratios: buildRatios(nodeSummary, boaSummary),
  };
}

async function main() {
  try {
    mkdirSync(artifactDir, { recursive: true });

    const bundleEnv = await bundleExampleBackends();
    const summary = isSweepMode
      ? runBenchmarkSweep(bundleEnv)
      : buildSingleSummary(runNodeBaseline(), runBoaBaseline(bundleEnv));

    writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');

    console.log('[bench] Wrote benchmark summary:', summaryPath);
    console.log(`[bench] Boa profile: ${isReleaseMode ? 'release' : 'test'}`);
    if (isSweepMode) {
      for (const profile of summary.profiles) {
        console.log(
          `[bench] ${profile.id} analytics backend ratio=${profile.ratios.analyticsStudio.backendDurationMs} roundTrip ratio=${profile.ratios.analyticsStudio.roundTripMs}`,
        );
        console.log(
          `[bench] ${profile.id} sync backend ratio=${profile.ratios.syncStorm.backendDurationMs} roundTrip ratio=${profile.ratios.syncStorm.roundTripMs}`,
        );
        console.log(
          `[bench] ${profile.id} workflow backend ratio=${profile.ratios.workflowLab.backendDurationMs} roundTrip ratio=${profile.ratios.workflowLab.roundTripMs}`,
        );
      }
    } else {
      console.log(
        `[bench] analytics backend ratio=${summary.ratios.analyticsStudio.backendDurationMs} roundTrip ratio=${summary.ratios.analyticsStudio.roundTripMs}`,
      );
      console.log(
        `[bench] sync backend ratio=${summary.ratios.syncStorm.backendDurationMs} roundTrip ratio=${summary.ratios.syncStorm.roundTripMs}`,
      );
      console.log(
        `[bench] workflow backend ratio=${summary.ratios.workflowLab.backendDurationMs} roundTrip ratio=${summary.ratios.workflowLab.roundTripMs}`,
      );
    }
  } finally {
    rmSync(bundleDir, { recursive: true, force: true });
  }
}

function runBenchmarkSweep(bundleEnv) {
  return {
    generatedAt: new Date().toISOString(),
    platform: process.platform,
    mode: 'headless-backend-runtime',
    boaProfile: isReleaseMode ? 'release' : 'test',
    sweep: true,
    profiles: benchmarkSweepProfiles.map((profile) => {
      const overrideEnv = {
        VOLT_BENCH_PROFILE_JSON: JSON.stringify(profile.config),
      };
      const nodeSummary = runNodeBaseline(overrideEnv);
      const boaSummary = runBoaBaseline(bundleEnv, overrideEnv);
      return {
        id: profile.id,
        config: profile.config,
        node: nodeSummary,
        boa: boaSummary,
        ratios: buildRatios(nodeSummary, boaSummary),
      };
    }),
  };
}

await main();
