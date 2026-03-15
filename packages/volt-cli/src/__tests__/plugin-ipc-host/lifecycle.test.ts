import { afterEach, describe, expect, it } from 'vitest';
import type { ChildProcess } from 'node:child_process';

import { PluginIpcHost } from '../../utils/plugin-ipc-host.js';

import { killProcess, spawnEcho } from './fixtures.js';

describe('PluginIpcHost lifecycle', () => {
  let host: PluginIpcHost;
  let proc: ChildProcess;

  afterEach(async () => {
    host?.stopHeartbeat();
    await killProcess(proc);
  });

  it('receives ready signal from plugin', async () => {
    host = new PluginIpcHost();
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);
  });

  it('sends a request and receives correlated response', async () => {
    host = new PluginIpcHost();
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    const response = await host.request('test.echo', { hello: 'world' });
    expect(response.type).toBe('response');
    expect(response.method).toBe('test.echo');
    expect(response.payload).toEqual({ hello: 'world' });
    expect(response.error).toBeNull();
  });

  it('handles multiple concurrent requests', async () => {
    host = new PluginIpcHost();
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    const results = await Promise.all([
      host.request('method.a', { n: 1 }),
      host.request('method.b', { n: 2 }),
      host.request('method.c', { n: 3 }),
    ]);

    expect(results[0].method).toBe('method.a');
    expect(results[1].method).toBe('method.b');
    expect(results[2].method).toBe('method.c');
  });

  it('heartbeat exchange works', async () => {
    host = new PluginIpcHost({ heartbeatIntervalMs: 100, heartbeatTimeoutMs: 200 });
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    let ackCount = 0;
    host.on('heartbeat-ack', () => ackCount++);
    host.startHeartbeat();

    await new Promise((resolve) => setTimeout(resolve, 350));
    host.stopHeartbeat();
    expect(ackCount).toBeGreaterThanOrEqual(2);
  });

  it('shuts down gracefully via deactivate signal', async () => {
    host = new PluginIpcHost();
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    await host.shutdown(3000);
    expect(proc.exitCode).toBe(0);
  });

  it('emits exit event when process ends', async () => {
    host = new PluginIpcHost();
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    const exitPromise = new Promise<number | null>((resolve) => {
      host.on('exit', (code: number | null) => resolve(code));
    });

    host.sendSignal('deactivate');
    expect(await exitPromise).toBe(0);
  });

  it('captures stderr output', async () => {
    const stderrScript = `
      const { Buffer } = require('buffer');
      function writeFrame(msg) {
        const body = Buffer.from(JSON.stringify(msg) + '\\n', 'utf-8');
        const header = Buffer.alloc(4);
        header.writeUInt32LE(body.length, 0);
        process.stdout.write(Buffer.concat([header, body]));
      }
      process.stderr.write('diagnostic output\\n');
      writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });
      process.stdin.on('end', () => process.exit(0));
    `;

    proc = spawnEcho();
    proc.kill('SIGKILL');
    proc = (await import('node:child_process')).spawn(process.execPath, ['-e', stderrScript], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    host = new PluginIpcHost();

    const stderrChunks: string[] = [];
    host.on('stderr', (data: string) => stderrChunks.push(data));
    host.attach(proc);
    await host.waitForReady(5000);

    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(stderrChunks.join('')).toContain('diagnostic output');
  });

  it('waitForReady cleans up listener on timeout', async () => {
    host = new PluginIpcHost();
    proc = (await import('node:child_process')).spawn(
      process.execPath,
      ['-e', "process.stdin.on('data', () => {}); process.stdin.on('end', () => process.exit(0));"],
      { stdio: ['pipe', 'pipe', 'pipe'] },
    );
    host.attach(proc);

    const listenerCountBefore = host.listenerCount('message');
    await expect(host.waitForReady(100)).rejects.toThrow(/ready signal/);
    expect(host.listenerCount('message')).toBe(listenerCountBefore);
  });
});
