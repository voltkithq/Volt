import { afterEach, describe, expect, it } from 'vitest';
import { spawn, type ChildProcess } from 'node:child_process';

import { PluginIpcHost, type IpcMessage } from '../../utils/plugin-ipc-host.js';

import { killProcess, spawnEcho, spawnSlowEcho } from './fixtures.js';

describe('PluginIpcHost flow control and failures', () => {
  let host: PluginIpcHost;
  let proc: ChildProcess;

  afterEach(async () => {
    host?.stopHeartbeat();
    await killProcess(proc);
  });

  it('request times out if no response', async () => {
    host = new PluginIpcHost({ callTimeoutMs: 200 });
    proc = spawn(
      process.execPath,
      [
        '-e',
        `
          const { Buffer } = require('buffer');
          function writeFrame(msg) {
            const body = Buffer.from(JSON.stringify(msg) + '\\n', 'utf-8');
            const header = Buffer.alloc(4);
            header.writeUInt32LE(body.length, 0);
            process.stdout.write(Buffer.concat([header, body]));
          }
          writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });
          process.stdin.on('data', () => {});
          process.stdin.on('end', () => process.exit(0));
        `,
      ],
      { stdio: ['pipe', 'pipe', 'pipe'] },
    );
    host.attach(proc);
    await host.waitForReady(5000);

    await expect(host.request('noop')).rejects.toThrow(/timed out/);
  });

  it('fails pending requests on process crash', async () => {
    host = new PluginIpcHost({ callTimeoutMs: 5000 });
    proc = spawn(
      process.execPath,
      [
        '-e',
        `
          const { Buffer } = require('buffer');
          function writeFrame(msg) {
            const body = Buffer.from(JSON.stringify(msg) + '\\n', 'utf-8');
            const header = Buffer.alloc(4);
            header.writeUInt32LE(body.length, 0);
            process.stdout.write(Buffer.concat([header, body]));
          }
          writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });
          process.stdin.on('data', () => process.exit(1));
        `,
      ],
      { stdio: ['pipe', 'pipe', 'pipe'] },
    );
    host.attach(proc);
    await host.waitForReady(5000);

    await expect(host.request('test')).rejects.toThrow(/PLUGIN_CRASHED/);
  });

  it('backpressure drops oldest queued request', async () => {
    host = new PluginIpcHost({ callTimeoutMs: 10000, maxInflight: 1, maxQueueDepth: 2 });
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    const promises: Promise<IpcMessage>[] = [
      host.request('r1'),
      host.request('r2'),
      host.request('r3'),
      host.request('r4'),
    ];

    const results = await Promise.allSettled(promises);
    expect(results[0].status).toBe('fulfilled');
    expect(results[1].status).toBe('rejected');
    if (results[1].status === 'rejected') {
      expect(results[1].reason.message).toBe('BACKPRESSURE');
    }
    expect(results[2].status).toBe('fulfilled');
    expect(results[3].status).toBe('fulfilled');
  });

  it('cancellation signal can be sent during a pending slow request', async () => {
    host = new PluginIpcHost({ callTimeoutMs: 5000 });
    proc = spawnSlowEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    const responsePromise = host.request('long.operation');
    host.cancel('some-request-id');
    const response = await responsePromise;
    expect(response.type).toBe('response');
    expect(response.method).toBe('long.operation');
  });

  it('inflight and queue counts are tracked', async () => {
    host = new PluginIpcHost({ maxInflight: 1, maxQueueDepth: 10 });
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    expect(host.inflightCount).toBe(0);
    expect(host.queueLength).toBe(0);

    const first = host.request('slow');
    expect(host.inflightCount).toBe(1);

    const second = host.request('queued');
    expect(host.queueLength).toBe(1);

    await Promise.all([first, second]);
    expect(host.inflightCount).toBe(0);
    expect(host.queueLength).toBe(0);
  });
});
