import { describe, it, expect, afterEach } from 'vitest';
import { ChildProcess, spawn } from 'node:child_process';
import { resolve } from 'node:path';
import { PluginIpcHost } from '../utils/plugin-ipc-host.js';

const BINARY_PATH = resolve(
  __dirname,
  '../../../../crates/volt-plugin-host/../../target/debug/volt-plugin-host.exe',
);

function makeConfig(overrides?: Record<string, unknown>): string {
  const config = {
    pluginId: 'test.plugin',
    capabilities: ['fs', 'http'],
    dataRoot: '.',
    ...overrides,
  };
  return Buffer.from(JSON.stringify(config)).toString('base64');
}

function spawnHost(configB64: string): ChildProcess {
  return spawn(BINARY_PATH, ['--plugin', '--config', configB64], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
}

describe('volt-plugin-host binary integration', () => {
  let host: PluginIpcHost;
  let proc: ChildProcess;

  afterEach(async () => {
    host?.stopHeartbeat();
    if (proc && proc.exitCode === null) {
      proc.kill('SIGKILL');
      await new Promise<void>((r) => proc.on('exit', () => r()));
    }
  });

  it('starts and sends ready signal', async () => {
    const config = makeConfig();
    proc = spawnHost(config);
    host = new PluginIpcHost();
    host.attach(proc);
    await host.waitForReady(10000);
  });

  it('responds to heartbeats', async () => {
    const config = makeConfig();
    proc = spawnHost(config);
    host = new PluginIpcHost({ heartbeatIntervalMs: 100, heartbeatTimeoutMs: 500 });
    host.attach(proc);
    await host.waitForReady(10000);

    let ackCount = 0;
    host.on('heartbeat-ack', () => ackCount++);
    host.startHeartbeat();

    await new Promise((r) => setTimeout(r, 400));
    host.stopHeartbeat();
    expect(ackCount).toBeGreaterThanOrEqual(2);
  });

  it('exits cleanly on deactivate signal', async () => {
    const config = makeConfig();
    proc = spawnHost(config);
    host = new PluginIpcHost();
    host.attach(proc);
    await host.waitForReady(10000);

    await host.shutdown(5000);
    expect(proc.exitCode).toBe(0);
  });

  it('exits cleanly on stdin EOF', async () => {
    const config = makeConfig();
    proc = spawnHost(config);
    host = new PluginIpcHost();
    host.attach(proc);
    await host.waitForReady(10000);

    proc.stdin!.end();
    const code = await new Promise<number | null>((resolve) => {
      proc.on('exit', (c) => resolve(c));
    });
    expect(code).toBe(0);
  });

  it('returns UNHANDLED error for unknown requests', async () => {
    const config = makeConfig();
    proc = spawnHost(config);
    host = new PluginIpcHost();
    host.attach(proc);
    await host.waitForReady(10000);

    const response = await host.request('nonexistent.method', { x: 1 });
    expect(response.type).toBe('response');
    expect(response.error).not.toBeNull();
    expect(response.error!.code).toBe('UNHANDLED');
  });

  it('exits with error if --plugin flag is missing', async () => {
    proc = spawn(BINARY_PATH, ['--config', makeConfig()], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    const code = await new Promise<number | null>((resolve) => {
      proc.on('exit', (c) => resolve(c));
    });
    expect(code).toBe(1);
  });

  it('exits with error if --config is missing', async () => {
    proc = spawn(BINARY_PATH, ['--plugin'], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    const code = await new Promise<number | null>((resolve) => {
      proc.on('exit', (c) => resolve(c));
    });
    expect(code).toBe(1);
  });

  it('accepts empty capabilities', async () => {
    const config = makeConfig({ capabilities: [] });
    proc = spawnHost(config);
    host = new PluginIpcHost();
    host.attach(proc);
    await host.waitForReady(10000);

    await host.shutdown(3000);
    expect(proc.exitCode).toBe(0);
  });
});
