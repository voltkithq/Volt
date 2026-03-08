import { loadSoakRuntimeConfig, soakTasks } from './soak-config.mjs';
import { runSoakTasks } from './soak-orchestration.mjs';
import { writeSoakArtifacts, writeStepSummary } from './soak-reporting.mjs';

async function main() {
  const config = loadSoakRuntimeConfig(process.env);
  const tasks = soakTasks();

  console.log(`[soak] Starting nightly soak: iterations=${config.iterations}, warmup=${config.warmup}`);
  console.log(`[soak] Metrics file: ${config.metricsFile}`);
  console.log(`[soak] Trend file: ${config.trendFile}`);
  console.log(`[soak] Report file: ${config.reportFile}`);

  const summary = await runSoakTasks(tasks, {
    iterations: config.iterations,
    warmup: config.warmup,
    platform: process.platform,
    arch: process.arch,
    nodeVersion: process.version,
  });

  await writeSoakArtifacts(summary, config);
  console.log(`[soak] Wrote metrics: ${config.metricsFile}`);
  console.log(`[soak] Wrote trend: ${config.trendFile}`);
  console.log(`[soak] Wrote report: ${config.reportFile}`);
  writeStepSummary(summary);

  if (!summary.passed) {
    console.error('[soak] Soak assertions failed.');
    process.exit(1);
  }

  console.log('[soak] Soak assertions passed.');
}

await main().catch((err) => {
  const message = err instanceof Error ? err.stack ?? err.message : String(err);
  console.error(`[soak] fatal error: ${message}`);
  process.exit(1);
});
