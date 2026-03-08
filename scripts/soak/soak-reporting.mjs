import { appendFileSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

export async function writeSoakArtifacts(summary, files) {
  const { metricsFile, trendFile, reportFile } = files;
  await mkdir(dirname(metricsFile), { recursive: true });
  await mkdir(dirname(trendFile), { recursive: true });
  await mkdir(dirname(reportFile), { recursive: true });
  await writeFile(metricsFile, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
  appendJsonLine(trendFile, {
    startedAt: summary.startedAt,
    finishedAt: summary.finishedAt,
    platform: summary.platform,
    arch: summary.arch,
    node: summary.node,
    iterations: summary.iterations,
    warmupEnabled: summary.warmupEnabled,
    totalDurationMs: summary.totalDurationMs,
    totalFailures: summary.totalFailures,
    failedTaskNames: summary.failedTaskNames,
    totalDroppedDispatchSignals: summary.totalDroppedDispatchSignals,
    taskP95Ms: summary.tasks.map((task) => ({
      name: task.name,
      p95Ms: task.p95Ms,
      thresholdMs: task.thresholdMs,
      completedIterations: task.completedIterations,
      passed: task.passed,
    })),
    passed: summary.passed,
  });
  await writeFile(reportFile, buildSoakReport(summary), 'utf8');
}

export function writeStepSummary(summary) {
  const stepSummaryPath = process.env.GITHUB_STEP_SUMMARY;
  if (!stepSummaryPath) {
    return;
  }

  const lines = [];
  lines.push('## Nightly Soak Summary');
  lines.push('');
  lines.push(`- Platform: \`${summary.platform}/${summary.arch}\``);
  lines.push(`- Iterations per task: \`${summary.iterations}\``);
  lines.push(`- Warmup enabled: \`${summary.warmupEnabled}\``);
  lines.push(`- Total duration: \`${summary.totalDurationMs}ms\``);
  lines.push(`- Passed: \`${summary.passed}\``);
  lines.push(`- Failed tasks: \`${summary.totalFailures}\``);
  lines.push(`- Dropped dispatch signals: \`${summary.totalDroppedDispatchSignals}\``);
  lines.push('');
  lines.push('| Task | p95(ms) | Threshold(ms) | Trend Δ(ms) | Avg(ms) | Max(ms) | Failure Reasons | Dropped Dispatch | Passed |');
  lines.push('|---|---:|---:|---:|---:|---:|---:|---:|---|');
  for (const task of summary.tasks) {
    lines.push(
      `| ${task.name} | ${task.p95Ms} | ${task.thresholdMs} | ${task.p95TrendDeltaMs} | ${task.avgMs} | ${task.maxMs} | ${formatFailureReasons(task)} | ${task.droppedDispatchSignals} | ${task.passed} |`,
    );
  }

  if (summary.failedTaskNames.length > 0) {
    lines.push('');
    lines.push('### Failed Tasks');
    for (const taskName of summary.failedTaskNames) {
      lines.push(`- ${taskName}`);
    }
  }

  // The GitHub runner appends this file to job summary.
  appendFileSync(stepSummaryPath, `${lines.join('\n')}\n`, 'utf8');
}

export function buildSoakReport(summary) {
  const lines = [];
  lines.push('# Soak Report');
  lines.push('');
  lines.push(`- Started: ${summary.startedAt}`);
  lines.push(`- Finished: ${summary.finishedAt}`);
  lines.push(`- Platform: ${summary.platform}/${summary.arch}`);
  lines.push(`- Node: ${summary.node}`);
  lines.push(`- Iterations: ${summary.iterations}`);
  lines.push(`- Warmup: ${summary.warmupEnabled}`);
  lines.push(`- Passed: ${summary.passed}`);
  lines.push(`- Failed tasks: ${summary.totalFailures}`);
  lines.push(`- Dropped dispatch signals: ${summary.totalDroppedDispatchSignals}`);
  if (summary.iterations < 8) {
    lines.push('- Note: Trend metrics are low-confidence with fewer than 8 samples.');
  }
  lines.push('');
  lines.push('| Task | p95(ms) | Threshold(ms) | Trend Δ(ms) | Avg(ms) | Max(ms) | Passed |');
  lines.push('|---|---:|---:|---:|---:|---:|---|');
  for (const task of summary.tasks) {
    lines.push(
      `| ${task.name} | ${task.p95Ms} | ${task.thresholdMs} | ${task.p95TrendDeltaMs} | ${task.avgMs} | ${task.maxMs} | ${task.passed} |`,
    );
  }

  const failedTasks = summary.tasks.filter((task) => !task.passed);
  if (failedTasks.length > 0) {
    lines.push('');
    lines.push('## Failure Details');
    for (const task of failedTasks) {
      lines.push('');
      lines.push(`### ${task.name}`);
      lines.push(`- Failure reasons: ${task.failureReasons.join(', ')}`);
      lines.push(`- Threshold: ${task.thresholdMs}ms`);
      lines.push(`- p95: ${task.p95Ms}ms`);
      for (const failure of task.failures) {
        const runLabel = failure.run === 0 ? 'warmup' : `run ${failure.run}`;
        lines.push(
          `- ${runLabel}: exit=${failure.exitCode}, duration=${failure.durationMs}ms, reason=${failure.reason}`,
        );
        if (failure.outputTail) {
          lines.push('```text');
          lines.push(failure.outputTail);
          lines.push('```');
        }
      }
    }
  }

  return `${lines.join('\n')}\n`;
}

function appendJsonLine(file, value) {
  appendFileSync(file, `${JSON.stringify(value)}\n`, 'utf8');
}

function formatFailureReasons(task) {
  if (!Array.isArray(task.failureReasons) || task.failureReasons.length === 0) {
    return 'none';
  }
  return task.failureReasons.join(', ');
}
