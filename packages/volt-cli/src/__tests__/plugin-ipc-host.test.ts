import { describe, it, expect, afterEach } from 'vitest';
import { ChildProcess, spawn } from 'node:child_process';
import { resolve } from 'node:path';
import {
  PluginIpcHost,
  frameMessage,
  tryParseFrame,
  type IpcMessage,
} from '../utils/plugin-ipc-host.js';

// ── Mock echo process ────────────────────────────────────────────
// A tiny Node.js script that acts as a mock plugin process:
// - On start, sends a "ready" signal
// - Echoes requests back as responses
// - Responds to heartbeats with heartbeat-ack
// - Exits on "deactivate" signal

const ECHO_SCRIPT = `
const { Buffer } = require('buffer');

function writeFrame(msg) {
  const json = JSON.stringify(msg);
  const body = Buffer.from(json + '\\n', 'utf-8');
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  process.stdout.write(Buffer.concat([header, body]));
}

// Send ready signal
writeFrame({
  type: 'signal',
  id: 'init',
  method: 'ready',
  payload: null,
  error: null,
});

let buf = Buffer.alloc(0);
process.stdin.on('data', (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  while (buf.length >= 4) {
    const len = buf.readUInt32LE(0);
    if (buf.length < 4 + len) break;
    const raw = buf.subarray(4, 4 + len).toString('utf-8');
    const stripped = raw.endsWith('\\n') ? raw.slice(0, -1) : raw;
    const msg = JSON.parse(stripped);
    buf = buf.subarray(4 + len);

    if (msg.type === 'signal' && msg.method === 'heartbeat') {
      writeFrame({
        type: 'signal',
        id: msg.id,
        method: 'heartbeat-ack',
        payload: null,
        error: null,
      });
    } else if (msg.type === 'signal' && msg.method === 'deactivate') {
      process.exit(0);
    } else if (msg.type === 'request') {
      writeFrame({
        type: 'response',
        id: msg.id,
        method: msg.method,
        payload: msg.payload,
        error: null,
      });
    }
  }
});

process.stdin.on('end', () => process.exit(0));
`;

function spawnEcho(): ChildProcess {
  return spawn(process.execPath, ['-e', ECHO_SCRIPT], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
}

// ── Framing unit tests ───────────────────────────────────────────

describe('frameMessage / tryParseFrame', () => {
  it('roundtrips a message through frame/parse', () => {
    const msg: IpcMessage = {
      type: 'request',
      id: 'abc-123',
      method: 'test.echo',
      payload: { key: 'value' },
      error: null,
    };
    const frame = frameMessage(msg);
    const parsed = tryParseFrame(frame, 0);
    expect(parsed).not.toBeNull();
    expect(parsed!.message).toEqual(msg);
    expect(parsed!.bytesConsumed).toBe(frame.length);
  });

  it('returns null for incomplete header', () => {
    expect(tryParseFrame(Buffer.alloc(2), 0)).toBeNull();
  });

  it('returns null for incomplete body', () => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(100, 0); // claims 100 bytes but buffer is only 4
    expect(tryParseFrame(buf, 0)).toBeNull();
  });

  it('parses multiple frames from a single buffer', () => {
    const msg1: IpcMessage = {
      type: 'event',
      id: '1',
      method: 'a',
      payload: null,
      error: null,
    };
    const msg2: IpcMessage = {
      type: 'event',
      id: '2',
      method: 'b',
      payload: null,
      error: null,
    };
    const combined = Buffer.concat([frameMessage(msg1), frameMessage(msg2)]);

    const first = tryParseFrame(combined, 0);
    expect(first).not.toBeNull();
    expect(first!.message.id).toBe('1');

    const second = tryParseFrame(combined, first!.bytesConsumed);
    expect(second).not.toBeNull();
    expect(second!.message.id).toBe('2');
  });

  it('rejects zero-length frame', () => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(0, 0);
    expect(tryParseFrame(buf, 0)).toBeNull();
  });

  it('rejects oversized frame (>16MB)', () => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(17 * 1024 * 1024, 0);
    expect(tryParseFrame(buf, 0)).toBeNull();
  });
});

// ── Integration tests with mock process ──────────────────────────

describe('PluginIpcHost with echo process', () => {
  let host: PluginIpcHost;
  let proc: ChildProcess;

  afterEach(async () => {
    host?.stopHeartbeat();
    if (proc && proc.exitCode === null) {
      proc.kill('SIGKILL');
      await new Promise<void>((r) => proc.on('exit', () => r()));
    }
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
    expect(results[0].payload).toEqual({ n: 1 });
    expect(results[1].payload).toEqual({ n: 2 });
    expect(results[2].payload).toEqual({ n: 3 });
  });

  it('heartbeat exchange works', async () => {
    host = new PluginIpcHost({ heartbeatIntervalMs: 100, heartbeatTimeoutMs: 200 });
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    let ackCount = 0;
    host.on('heartbeat-ack', () => ackCount++);
    host.startHeartbeat();

    await new Promise((r) => setTimeout(r, 350));
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
    const code = await exitPromise;
    expect(code).toBe(0);
  });

  it('request times out if no response', async () => {
    host = new PluginIpcHost({ callTimeoutMs: 200 });
    // Spawn a process that reads but never responds to requests
    const silentScript = `
      const { Buffer } = require('buffer');
      function writeFrame(msg) {
        const json = JSON.stringify(msg);
        const body = Buffer.from(json + '\\n', 'utf-8');
        const header = Buffer.alloc(4);
        header.writeUInt32LE(body.length, 0);
        process.stdout.write(Buffer.concat([header, body]));
      }
      writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });
      process.stdin.on('data', () => {}); // consume but never respond
      process.stdin.on('end', () => process.exit(0));
    `;
    proc = spawn(process.execPath, ['-e', silentScript], { stdio: ['pipe', 'pipe', 'pipe'] });
    host.attach(proc);
    await host.waitForReady(5000);

    await expect(host.request('noop')).rejects.toThrow(/timed out/);
  });

  it('fails pending requests on process crash', async () => {
    host = new PluginIpcHost({ callTimeoutMs: 5000 });
    // Spawn a process that dies after ready
    const crashScript = `
      const { Buffer } = require('buffer');
      function writeFrame(msg) {
        const json = JSON.stringify(msg);
        const body = Buffer.from(json + '\\n', 'utf-8');
        const header = Buffer.alloc(4);
        header.writeUInt32LE(body.length, 0);
        process.stdout.write(Buffer.concat([header, body]));
      }
      writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });
      process.stdin.on('data', () => {
        process.exit(1);
      });
    `;
    proc = spawn(process.execPath, ['-e', crashScript], { stdio: ['pipe', 'pipe', 'pipe'] });
    host.attach(proc);
    await host.waitForReady(5000);

    await expect(host.request('test')).rejects.toThrow(/PLUGIN_CRASHED/);
  });

  it('backpressure drops oldest queued request', async () => {
    host = new PluginIpcHost({
      callTimeoutMs: 10000,
      maxInflight: 1,
      maxQueueDepth: 2,
    });
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    const promises: Promise<IpcMessage>[] = [];
    // First request takes the inflight slot
    promises.push(host.request('r1'));
    // Next two fill the queue
    promises.push(host.request('r2'));
    promises.push(host.request('r3'));
    // Fourth should drop r2 (oldest in queue)
    promises.push(host.request('r4'));

    const results = await Promise.allSettled(promises);
    // r1 succeeds (inflight)
    expect(results[0].status).toBe('fulfilled');
    // r2 was dropped by backpressure
    expect(results[1].status).toBe('rejected');
    if (results[1].status === 'rejected') {
      expect(results[1].reason.message).toBe('BACKPRESSURE');
    }
    // r3 and r4 should eventually succeed
    expect(results[2].status).toBe('fulfilled');
    expect(results[3].status).toBe('fulfilled');
  });

  it('cancellation signal can be sent for a pending request', async () => {
    host = new PluginIpcHost();
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    // The echo process doesn't handle cancellation specially — it echoes the request anyway.
    // This just tests that cancel() sends a signal without error.
    const responsePromise = host.request('long.operation');
    host.cancel((await responsePromise).id);
    // If we get here without throwing, cancel didn't break anything.
  });

  it('captures stderr output', async () => {
    const stderrScript = `
      const { Buffer } = require('buffer');
      function writeFrame(msg) {
        const json = JSON.stringify(msg);
        const body = Buffer.from(json + '\\n', 'utf-8');
        const header = Buffer.alloc(4);
        header.writeUInt32LE(body.length, 0);
        process.stdout.write(Buffer.concat([header, body]));
      }
      process.stderr.write('diagnostic output\\n');
      writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });
      process.stdin.on('end', () => process.exit(0));
    `;
    proc = spawn(process.execPath, ['-e', stderrScript], { stdio: ['pipe', 'pipe', 'pipe'] });
    host = new PluginIpcHost();

    const stderrChunks: string[] = [];
    host.on('stderr', (data: string) => stderrChunks.push(data));
    host.attach(proc);
    await host.waitForReady(5000);

    // Give stderr a moment to arrive
    await new Promise((r) => setTimeout(r, 100));
    expect(stderrChunks.join('')).toContain('diagnostic output');
  });

  it('inflight and queue counts are tracked', async () => {
    host = new PluginIpcHost({ maxInflight: 1, maxQueueDepth: 10 });
    proc = spawnEcho();
    host.attach(proc);
    await host.waitForReady(5000);

    expect(host.inflightCount).toBe(0);
    expect(host.queueLength).toBe(0);

    // Don't await yet — just fire
    const p1 = host.request('slow');
    // r1 is inflight, no queue yet
    expect(host.inflightCount).toBe(1);

    const p2 = host.request('queued');
    expect(host.queueLength).toBe(1);

    // Wait for both to complete
    await Promise.all([p1, p2]);
    expect(host.inflightCount).toBe(0);
    expect(host.queueLength).toBe(0);
  });
});
