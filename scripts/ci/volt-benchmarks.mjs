import { mkdirSync, rmSync, writeFileSync } from 'node:fs';

import {
  artifactDir,
  bundleDir,
  isReleaseMode,
  isSweepMode,
  summaryPath,
} from './volt-benchmarks/config.mjs';
import { logSummary } from './volt-benchmarks/logging.mjs';
import { bundleExampleBackends, runBenchmarkSummary } from './volt-benchmarks/runner.mjs';

async function main() {
  try {
    mkdirSync(artifactDir, { recursive: true });
    const bundleEnv = await bundleExampleBackends(bundleDir);
    const summary = runBenchmarkSummary(bundleEnv, isSweepMode);

    writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
    logSummary(summary, isReleaseMode, isSweepMode, summaryPath);
  } finally {
    rmSync(bundleDir, { recursive: true, force: true });
  }
}

await main();
