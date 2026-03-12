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

function runBoaVariant(bundleEnv, engine, extraEnv = {}) {
  return runBoaBaseline(bundleEnv, {
    ...extraEnv,
    VOLT_BENCH_ENGINE: engine,
  });
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

function buildSpeedups(jsSummary, nativeSummary) {
  return {
    analyticsStudio: {
      backendDurationMs: ratio(
        metric(jsSummary.analyticsStudio, 'backendDurationMs'),
        metric(nativeSummary.analyticsStudio, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(jsSummary.analyticsStudio, 'roundTripMs'),
        metric(nativeSummary.analyticsStudio, 'roundTripMs'),
      ),
    },
    syncStorm: {
      backendDurationMs: ratio(
        metric(jsSummary.syncStorm, 'backendDurationMs'),
        metric(nativeSummary.syncStorm, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(jsSummary.syncStorm, 'roundTripMs'),
        metric(nativeSummary.syncStorm, 'roundTripMs'),
      ),
    },
    workflowLab: {
      backendDurationMs: ratio(
        metric(jsSummary.workflowLab, 'backendDurationMs'),
        metric(nativeSummary.workflowLab, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(jsSummary.workflowLab, 'roundTripMs'),
        metric(nativeSummary.workflowLab, 'roundTripMs'),
      ),
    },
  };
}

function buildVariantSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary) {
  return {
    node: nodeSummary,
    boaJs: boaJsSummary,
    boaNative: boaNativeSummary,
    directNative: directNativeSummary,
    ratios: {
      boaJsVsNode: buildRatios(nodeSummary, boaJsSummary),
      boaNativeVsNode: buildRatios(nodeSummary, boaNativeSummary),
      directNativeVsNode: buildRatios(nodeSummary, directNativeSummary),
      forwardedNativeSpeedup: buildSpeedups(boaJsSummary, boaNativeSummary),
      directNativeSpeedup: buildSpeedups(boaJsSummary, directNativeSummary),
      directVsForwardedSpeedup: buildSpeedups(boaNativeSummary, directNativeSummary),
    },
  };
}

function buildSingleSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary) {
  return {
    generatedAt: new Date().toISOString(),
    platform: process.platform,
    mode: 'headless-backend-runtime',
    boaProfile: isReleaseMode ? 'release' : 'test',
    ...buildVariantSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary),
  };
}

async function main() {
  try {
    mkdirSync(artifactDir, { recursive: true });

    const bundleEnv = await bundleExampleBackends();
    const summary = isSweepMode
      ? runBenchmarkSweep(bundleEnv)
      : buildSingleSummary(
        runNodeBaseline(),
        runBoaVariant(bundleEnv, 'js'),
        runBoaVariant(bundleEnv, 'native'),
        runBoaVariant(bundleEnv, 'direct'),
      );

    writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');

    console.log('[bench] Wrote benchmark summary:', summaryPath);
    console.log(`[bench] Boa profile: ${isReleaseMode ? 'release' : 'test'}`);
    if (isSweepMode) {
      for (const profile of summary.profiles) {
        console.log(
          `[bench] ${profile.id} analytics js ratio=${profile.ratios.boaJsVsNode.analyticsStudio.backendDurationMs} forwarded ratio=${profile.ratios.boaNativeVsNode.analyticsStudio.backendDurationMs} direct ratio=${profile.ratios.directNativeVsNode.analyticsStudio.backendDurationMs} direct speedup=${profile.ratios.directNativeSpeedup.analyticsStudio.backendDurationMs}`,
        );
        console.log(
          `[bench] ${profile.id} sync js ratio=${profile.ratios.boaJsVsNode.syncStorm.backendDurationMs} forwarded ratio=${profile.ratios.boaNativeVsNode.syncStorm.backendDurationMs} direct ratio=${profile.ratios.directNativeVsNode.syncStorm.backendDurationMs} direct speedup=${profile.ratios.directNativeSpeedup.syncStorm.backendDurationMs}`,
        );
        console.log(
          `[bench] ${profile.id} workflow js ratio=${profile.ratios.boaJsVsNode.workflowLab.backendDurationMs} forwarded ratio=${profile.ratios.boaNativeVsNode.workflowLab.backendDurationMs} direct ratio=${profile.ratios.directNativeVsNode.workflowLab.backendDurationMs} direct speedup=${profile.ratios.directNativeSpeedup.workflowLab.backendDurationMs}`,
        );
      }
    } else {
      console.log(
        `[bench] analytics js ratio=${summary.ratios.boaJsVsNode.analyticsStudio.backendDurationMs} forwarded ratio=${summary.ratios.boaNativeVsNode.analyticsStudio.backendDurationMs} direct ratio=${summary.ratios.directNativeVsNode.analyticsStudio.backendDurationMs} direct speedup=${summary.ratios.directNativeSpeedup.analyticsStudio.backendDurationMs}`,
      );
      console.log(
        `[bench] sync js ratio=${summary.ratios.boaJsVsNode.syncStorm.backendDurationMs} forwarded ratio=${summary.ratios.boaNativeVsNode.syncStorm.backendDurationMs} direct ratio=${summary.ratios.directNativeVsNode.syncStorm.backendDurationMs} direct speedup=${summary.ratios.directNativeSpeedup.syncStorm.backendDurationMs}`,
      );
      console.log(
        `[bench] workflow js ratio=${summary.ratios.boaJsVsNode.workflowLab.backendDurationMs} forwarded ratio=${summary.ratios.boaNativeVsNode.workflowLab.backendDurationMs} direct ratio=${summary.ratios.directNativeVsNode.workflowLab.backendDurationMs} direct speedup=${summary.ratios.directNativeSpeedup.workflowLab.backendDurationMs}`,
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
      const boaJsSummary = runBoaVariant(bundleEnv, 'js', overrideEnv);
      const boaNativeSummary = runBoaVariant(bundleEnv, 'native', overrideEnv);
      const directNativeSummary = runBoaVariant(bundleEnv, 'direct', overrideEnv);
      return {
        id: profile.id,
        config: profile.config,
        ...buildVariantSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary),
      };
    }),
  };
}

await main();
