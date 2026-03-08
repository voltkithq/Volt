import { spawn } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import {
  DROPPED_DISPATCH_PATTERN,
  FAILURE_OUTPUT_TAIL_LINES,
  thresholdForPlatform,
} from './soak-config.mjs';

export async function runSoakTasks(tasks, options) {
  const {
    iterations,
    warmup,
    platform,
    arch,
    nodeVersion,
    startedAt = new Date().toISOString(),
  } = options;
  const runStartedAt = performance.now();
  const taskResults = [];
  const failedTaskNames = [];
  let totalDroppedDispatchSignals = 0;

  for (const task of tasks) {
    const durations = [];
    const runs = [];
    const failures = [];
    const thresholdMs = thresholdForPlatform(task, platform);
    let droppedDispatchSignals = 0;
    let warmupResult = null;

    if (warmup) {
      console.log(`[soak] Task=${task.name} warmup`);
      warmupResult = await runCommand(task.command, task.args);
      if (warmupResult.exitCode !== 0) {
        failures.push({
          run: 0,
          exitCode: warmupResult.exitCode,
          durationMs: warmupResult.durationMs,
          reason: 'warmup-non-zero-exit',
          outputTail: tailLines(warmupResult.output, FAILURE_OUTPUT_TAIL_LINES),
        });
      }
      droppedDispatchSignals += countPattern(warmupResult.output, DROPPED_DISPATCH_PATTERN);
    }

    for (let run = 1; run <= iterations; run += 1) {
      console.log(`[soak] Task=${task.name} run=${run}/${iterations}`);
      const result = await runCommand(task.command, task.args);
      const runDroppedDispatchSignals = countPattern(result.output, DROPPED_DISPATCH_PATTERN);
      durations.push(result.durationMs);
      droppedDispatchSignals += runDroppedDispatchSignals;

      runs.push({
        run,
        durationMs: result.durationMs,
        exitCode: result.exitCode,
        droppedDispatchSignals: runDroppedDispatchSignals,
        passed: result.exitCode === 0 && runDroppedDispatchSignals === 0,
      });

      if (result.exitCode !== 0) {
        failures.push({
          run,
          exitCode: result.exitCode,
          durationMs: result.durationMs,
          reason: 'non-zero-exit',
          outputTail: tailLines(result.output, FAILURE_OUTPUT_TAIL_LINES),
        });
      }
    }

    const p95Ms = percentile(durations, 0.95);
    const maxMs = Math.max(...durations);
    const minMs = Math.min(...durations);
    const avgMs = average(durations);
    const p95TrendDeltaMs = trendDelta(durations);
    const thresholdBreached = p95Ms > thresholdMs;
    const failureReasons = [];
    if (failures.length > 0) {
      failureReasons.push('non-zero-exit');
    }
    if (thresholdBreached) {
      failureReasons.push(`p95>${thresholdMs}ms`);
    }
    if (droppedDispatchSignals > 0) {
      failureReasons.push('dropped-dispatch-signals');
    }
    const taskFailed = failureReasons.length > 0;
    if (taskFailed) {
      failedTaskNames.push(task.name);
    }

    totalDroppedDispatchSignals += droppedDispatchSignals;
    taskResults.push({
      name: task.name,
      command: [task.command, ...task.args].join(' '),
      iterations,
      completedIterations: runs.length,
      warmupEnabled: warmup,
      warmupDurationMs: warmupResult?.durationMs ?? null,
      warmupExitCode: warmupResult?.exitCode ?? null,
      thresholdMs,
      p95Ms,
      minMs,
      avgMs,
      maxMs,
      p95TrendDeltaMs,
      runs,
      failures,
      failureReasons,
      thresholdBreached,
      droppedDispatchSignals,
      passed: !taskFailed,
    });
  }

  return {
    startedAt,
    finishedAt: new Date().toISOString(),
    platform,
    arch,
    node: nodeVersion,
    iterations,
    warmupEnabled: warmup,
    thresholdsByPlatform: taskResults.map((task) => ({
      name: task.name,
      thresholdMs: task.thresholdMs,
    })),
    totalDurationMs: Math.round(performance.now() - runStartedAt),
    totalFailures: failedTaskNames.length,
    failedTaskNames,
    totalDroppedDispatchSignals,
    tasks: taskResults,
    passed: failedTaskNames.length === 0,
  };
}

export async function runCommand(command, args) {
  const startedAt = performance.now();
  const child = spawnCommand(command, args);

  let output = '';
  child.stdout.on('data', (chunk) => {
    const text = chunk.toString();
    output += text;
    process.stdout.write(text);
  });
  child.stderr.on('data', (chunk) => {
    const text = chunk.toString();
    output += text;
    process.stderr.write(text);
  });

  const exitCode = await new Promise((resolve) => {
    let settled = false;
    child.on('error', (err) => {
      output += `${String(err)}\n`;
      if (!settled) {
        settled = true;
        resolve(1);
      }
    });
    child.on('close', (code) => {
      if (!settled) {
        settled = true;
        resolve(code);
      }
    });
  });

  return {
    exitCode: Number(exitCode ?? 1),
    durationMs: Math.round(performance.now() - startedAt),
    output,
  };
}

function spawnCommand(command, args) {
  if (process.platform === 'win32' && command === 'pnpm') {
    const commandLine = buildWindowsCommandLine(command, args);
    return spawn('cmd.exe', ['/d', '/s', '/c', commandLine], {
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });
  }

  return spawn(command, args, {
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: false,
    windowsHide: true,
  });
}

function buildWindowsCommandLine(command, args) {
  return [command, ...args.map(quoteWindowsArg)].join(' ');
}

function quoteWindowsArg(arg) {
  if (!/[ \t"&|<>^()]/.test(arg)) {
    return arg;
  }
  return `"${arg.replace(/"/g, '""')}"`;
}

function average(values) {
  if (values.length === 0) {
    return 0;
  }
  return Math.round(values.reduce((sum, value) => sum + value, 0) / values.length);
}

function trendDelta(values) {
  if (values.length < 2) {
    return 0;
  }
  const midpoint = Math.floor(values.length / 2);
  const first = values.slice(0, midpoint);
  const second = values.slice(midpoint);
  return average(second) - average(first);
}

function percentile(values, ratio) {
  if (values.length === 0) {
    return 0;
  }
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(ratio * sorted.length) - 1));
  return sorted[index];
}

function countPattern(value, pattern) {
  if (!value.includes(pattern)) {
    return 0;
  }

  // Parse numeric counters from logs like:
  // "[volt] Dropped native event callback dispatches: 2"
  const numericPattern = new RegExp(`${escapeRegExp(pattern)}\\s*(\\d+)`, 'g');
  let total = 0;
  let sawNumericCounter = false;
  for (const match of value.matchAll(numericPattern)) {
    const parsed = Number.parseInt(match[1], 10);
    if (Number.isFinite(parsed)) {
      total += parsed;
      sawNumericCounter = true;
    }
  }

  if (sawNumericCounter) {
    return total;
  }

  // Fallback for legacy logs that only include the marker text.
  return value.split(pattern).length - 1;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function tailLines(value, maxLines) {
  const lines = value.split(/\r?\n/).filter((line) => line.trim().length > 0);
  if (lines.length <= maxLines) {
    return lines.join('\n');
  }
  return lines.slice(lines.length - maxLines).join('\n');
}
