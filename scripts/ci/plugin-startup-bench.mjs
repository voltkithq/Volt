import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { spawn } from 'node:child_process';

const repoRoot = path.resolve(import.meta.dirname, '..', '..');
const defaultIterations = 10;
const spawnReadyThresholdMs = 200, firstResponseThresholdMs = 500;

function parseIterations(argv) {
  const flag = argv.find((arg) => arg.startsWith('--iterations='));
  if (!flag) {
    return defaultIterations;
  }
  const value = Number.parseInt(flag.split('=')[1] ?? '', 10);
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`invalid --iterations value: ${flag}`);
  }
  return value;
}

function resolvePluginHostBinary() {
  const binaryName = process.platform === 'win32' ? 'volt-plugin-host.exe' : 'volt-plugin-host';
  return path.join(repoRoot, 'target', 'release', binaryName);
}

function encodeMessage(message) {
  const body = Buffer.from(`${JSON.stringify(message)}\n`, 'utf8');
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  return Buffer.concat([header, body]);
}

function createFrameReader(onMessage) {
  let buffer = Buffer.alloc(0);
  return (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (buffer.length >= 4) {
      const length = buffer.readUInt32LE(0);
      if (buffer.length < 4 + length) {
        return;
      }
      const body = buffer.subarray(4, 4 + length).toString('utf8').trimEnd();
      buffer = buffer.subarray(4 + length);
      onMessage(JSON.parse(body));
    }
  };
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0
    ? (sorted[middle - 1] + sorted[middle]) / 2
    : sorted[middle];
}

function percentile95(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.ceil(0.95 * sorted.length) - 1;
  return sorted[Math.max(0, Math.min(index, sorted.length - 1))];
}

function buildSummary(values) {
  return {
    min: Math.min(...values),
    median: median(values),
    max: Math.max(...values),
    p95: percentile95(values),
  };
}

function createMinimalPlugin(tempRoot) {
  const backendEntry = path.join(tempRoot, 'plugin.js');
  const dataRoot = path.join(tempRoot, 'data');
  mkdirSync(dataRoot, { recursive: true });
  writeFileSync(
    backendEntry,
    [
      "import { definePlugin } from 'volt:plugin';",
      'definePlugin({',
      '  async activate(context) {',
      "    context.commands.register('ping', async (args) => ({ ok: true, echoed: args ?? null }));",
      '  },',
      '  async deactivate() {}',
      '});',
      '',
    ].join('\n'),
    'utf8',
  );
  return { backendEntry, dataRoot };
}

function waitForExit(child) {
  return new Promise((resolve, reject) => {
    child.once('exit', (code, signal) => resolve({ code, signal }));
    child.once('error', reject);
  });
}

async function runIteration(binary, iteration) {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), 'volt-plugin-bench-'));
  const { backendEntry, dataRoot } = createMinimalPlugin(tempRoot);
  const pluginId = `bench.plugin.${iteration}`;
  const config = {
    pluginId,
    backendEntry,
    manifest: {
      id: pluginId,
      name: 'Bench Plugin',
      version: '0.0.0',
      apiVersion: 1,
      engine: { volt: '>=0.1.0' },
      backend: './plugin.js',
      capabilities: [],
    },
    capabilities: [],
    dataRoot,
    delegatedGrants: [],
    hostIpcSettings: {
      heartbeatIntervalMs: 5000,
      heartbeatTimeoutMs: 3000,
      callTimeoutMs: 30000,
      maxInflight: 64,
      maxQueueDepth: 256,
    },
  };

  const child = spawn(
    binary,
    ['--plugin', '--config', Buffer.from(JSON.stringify(config), 'utf8').toString('base64')],
    { cwd: repoRoot, stdio: ['pipe', 'pipe', 'pipe'] },
  );

  const start = performance.now();
  let stderr = '';
  const pending = new Map();
  let readyResolve;
  let readyReject;
  const readyPromise = new Promise((resolve, reject) => {
    readyResolve = resolve;
    readyReject = reject;
  });

  child.stderr.on('data', (chunk) => {
    stderr += chunk.toString('utf8');
  });
  child.once('error', readyReject);
  child.once('exit', (code, signal) => {
    const error = new Error(`plugin host exited early code=${code} signal=${signal} stderr=${stderr}`);
    readyReject(error);
    for (const waiter of pending.values()) {
      waiter.reject(error);
    }
    pending.clear();
  });

  child.stdout.on('data', createFrameReader((message) => {
    if (message.type === 'signal' && message.method === 'ready') {
      readyResolve({ time: performance.now() });
      return;
    }
    if (message.type === 'request' && message.method === 'plugin:register-command') {
      child.stdin.write(
        encodeMessage({
          type: 'response',
          id: message.id,
          method: message.method,
          payload: null,
          error: null,
        }),
      );
      return;
    }
    const waiter = pending.get(message.id);
    if (!waiter) {
      return;
    }
    pending.delete(message.id);
    waiter.resolve({ message, time: performance.now() });
  }));

  function sendAndWait(message, timeoutMs = 30000) {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        if (pending.delete(message.id)) {
          reject(new Error(`timeout waiting for ${message.method}; stderr=${stderr}`));
        }
      }, timeoutMs);
      pending.set(message.id, {
        resolve: (value) => {
          clearTimeout(timeout);
          resolve(value);
        },
        reject: (error) => {
          clearTimeout(timeout);
          reject(error);
        },
      });
      child.stdin.write(encodeMessage(message), (error) => {
        if (!error) {
          return;
        }
        clearTimeout(timeout);
        pending.delete(message.id);
        reject(error);
      });
    });
  }

  const readyTimeout = setTimeout(() => {
    readyReject(new Error(`timeout waiting for ready; stderr=${stderr}`));
    child.kill();
  }, 30000);

  try {
    const ready = await readyPromise;
    clearTimeout(readyTimeout);

    const activate = await sendAndWait({
      type: 'signal',
      id: 'activate-1',
      method: 'activate',
      payload: null,
      error: null,
    });
    const command = await sendAndWait({
      type: 'request',
      id: 'command-1',
      method: 'plugin:invoke-command',
      payload: { id: 'ping', args: { hello: 'world' } },
      error: null,
    });
    const exitPromise = waitForExit(child);
    child.stdin.write(
      encodeMessage({
        type: 'signal',
        id: 'deactivate-1',
        method: 'deactivate',
        payload: null,
        error: null,
      }),
    );
    const exit = await exitPromise;
    if (activate.message.error || command.message.error || exit.code !== 0) {
      throw new Error(`benchmark iteration failed stderr=${stderr}`);
    }

    return {
      spawnToReadyMs: ready.time - start,
      spawnToFirstResponseMs: command.time - start,
      readyToActivateAckMs: activate.time - ready.time,
      activateToCommandResponseMs: command.time - activate.time,
    };
  } finally {
    clearTimeout(readyTimeout);
    child.kill();
    rmSync(tempRoot, { recursive: true, force: true });
  }
}

async function main() {
  const iterations = parseIterations(process.argv.slice(2));
  const binary = resolvePluginHostBinary();
  if (!existsSync(binary)) {
    throw new Error(`missing release plugin host binary: ${binary}`);
  }
  const results = [];

  for (let index = 0; index < iterations; index += 1) {
    const result = await runIteration(binary, index + 1);
    results.push(result);
    console.log(
      `iteration ${index + 1}: spawn->ready=${result.spawnToReadyMs.toFixed(2)}ms ` +
        `spawn->first-response=${result.spawnToFirstResponseMs.toFixed(2)}ms`,
    );
  }

  const readyValues = results.map((result) => result.spawnToReadyMs);
  const firstResponseValues = results.map((result) => result.spawnToFirstResponseMs);
  const readySummary = buildSummary(readyValues);
  const firstResponseSummary = buildSummary(firstResponseValues);
  const summary = {
    thresholds: {
      spawnToReadyMs: spawnReadyThresholdMs,
      spawnToFirstResponseMs: firstResponseThresholdMs,
    },
    spawnToReadyMs: readySummary,
    spawnToFirstResponseMs: firstResponseSummary,
    trustGroupsRecommended:
      readySummary.p95 > spawnReadyThresholdMs ||
      firstResponseSummary.p95 > firstResponseThresholdMs,
    iterations: results,
  };

  console.log(JSON.stringify(summary, null, 2));
}

await main();
