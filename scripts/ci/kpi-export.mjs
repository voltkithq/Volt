#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { spawn } from 'node:child_process';

function printUsage() {
  console.log(
    'Usage: node scripts/ci/kpi-export.mjs --output <file> --metric <name> [--scope <scope>] -- <command...>',
  );
}

function parseArgs(argv) {
  const separator = argv.indexOf('--');
  const optionArgs = separator >= 0 ? argv.slice(0, separator) : argv;
  const commandArgs = separator >= 0 ? argv.slice(separator + 1) : [];

  let outputPath = '';
  let metricName = '';
  let scope = 'ci';

  for (let i = 0; i < optionArgs.length; i += 1) {
    const arg = optionArgs[i];
    if (arg === '--output') {
      outputPath = optionArgs[i + 1] ?? '';
      i += 1;
      continue;
    }
    if (arg === '--metric') {
      metricName = optionArgs[i + 1] ?? '';
      i += 1;
      continue;
    }
    if (arg === '--scope') {
      scope = optionArgs[i + 1] ?? scope;
      i += 1;
      continue;
    }
    if (arg === '--help' || arg === '-h') {
      printUsage();
      process.exit(0);
    }
    console.error(`Unknown argument: ${arg}`);
    printUsage();
    process.exit(2);
  }

  if (!outputPath || !metricName || commandArgs.length === 0) {
    printUsage();
    process.exit(2);
  }

  return { outputPath, metricName, scope, commandArgs };
}

function writeSnapshot(snapshot, outputPath) {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, `${JSON.stringify(snapshot, null, 2)}\n`, 'utf8');
}

function appendSummary(snapshot) {
  const summaryPath = process.env.GITHUB_STEP_SUMMARY;
  if (!summaryPath) {
    return;
  }

  const lines = [
    `### KPI Metric: ${snapshot.metric.name}`,
    '',
    `- Scope: ${snapshot.scope}`,
    `- Duration: ${snapshot.metric.valueMs} ms`,
    `- Success: ${snapshot.command.success ? 'yes' : 'no'}`,
    `- Output: \`${snapshot.outputPath}\``,
    '',
  ];
  writeFileSync(summaryPath, `${lines.join('\n')}\n`, { encoding: 'utf8', flag: 'a' });
}

async function runMeasuredCommand(commandArgs) {
  const [command, ...args] = commandArgs;
  const startedAt = Date.now();

  const exitCode = await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });

    child.once('error', reject);
    child.once('close', (code) => resolve(code ?? 1));
  });

  return {
    exitCode,
    durationMs: Date.now() - startedAt,
    command,
    args,
  };
}

async function main() {
  const { outputPath, metricName, scope, commandArgs } = parseArgs(process.argv.slice(2));
  const run = await runMeasuredCommand(commandArgs);

  const snapshot = {
    generatedAt: new Date().toISOString(),
    scope,
    metric: {
      name: metricName,
      valueMs: run.durationMs,
      unit: 'ms',
    },
    command: {
      executable: run.command,
      args: run.args,
      success: run.exitCode === 0,
      exitCode: run.exitCode,
    },
    ciContext: {
      sha: process.env.GITHUB_SHA ?? null,
      runId: process.env.GITHUB_RUN_ID ?? null,
      runAttempt: process.env.GITHUB_RUN_ATTEMPT ?? null,
      workflow: process.env.GITHUB_WORKFLOW ?? null,
      job: process.env.GITHUB_JOB ?? null,
      ref: process.env.GITHUB_REF ?? null,
    },
    outputPath,
  };

  writeSnapshot(snapshot, outputPath);
  appendSummary(snapshot);
  process.exit(run.exitCode);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
