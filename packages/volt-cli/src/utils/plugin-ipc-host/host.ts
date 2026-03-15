import { ChildProcess, spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { EventEmitter } from 'node:events';

import { DEFAULT_PLUGIN_HOST_OPTIONS, type PluginHostOptions } from './options.js';
import { frameMessage, tryParseFrame } from './framing.js';
import { HeartbeatMonitor } from './heartbeat.js';
import { RequestTracker } from './request-tracker.js';
import type { IpcMessage } from './types.js';

export class PluginIpcHost extends EventEmitter {
  private proc: ChildProcess | null = null;
  private readBuf = Buffer.alloc(0);
  private closed = false;

  private readonly tracker: RequestTracker;
  private readonly heartbeat: HeartbeatMonitor;

  constructor(opts: PluginHostOptions = {}) {
    super();

    const heartbeatIntervalMs =
      opts.heartbeatIntervalMs ?? DEFAULT_PLUGIN_HOST_OPTIONS.heartbeatIntervalMs;
    const heartbeatTimeoutMs =
      opts.heartbeatTimeoutMs ?? DEFAULT_PLUGIN_HOST_OPTIONS.heartbeatTimeoutMs;
    const callTimeoutMs = opts.callTimeoutMs ?? DEFAULT_PLUGIN_HOST_OPTIONS.callTimeoutMs;
    const maxInflight = opts.maxInflight ?? DEFAULT_PLUGIN_HOST_OPTIONS.maxInflight;
    const maxQueueDepth = opts.maxQueueDepth ?? DEFAULT_PLUGIN_HOST_OPTIONS.maxQueueDepth;

    this.tracker = new RequestTracker(callTimeoutMs, maxInflight, maxQueueDepth, (msg) =>
      this.writeMessage(msg),
    );
    this.heartbeat = new HeartbeatMonitor(
      heartbeatIntervalMs,
      heartbeatTimeoutMs,
      () => this.sendSignal('heartbeat'),
      () => {
        this.emit('unresponsive');
        this.kill();
      },
    );
  }

  spawn(command: string, args: string[]): void {
    if (this.proc) throw new Error('Already spawned');
    this.attach(spawn(command, args, { stdio: ['pipe', 'pipe', 'pipe'] }));
  }

  attach(proc: ChildProcess): void {
    if (this.proc) throw new Error('Already spawned');
    this.proc = proc;
    this.proc.stdout!.on('data', (chunk: Buffer) => this.onData(chunk));
    this.proc.stderr!.on('data', (chunk: Buffer) => this.emit('stderr', chunk.toString('utf-8')));
    this.proc.on('exit', (code, signal) => this.onProcessExit(code, signal));
  }

  waitForReady(timeoutMs = 10000): Promise<void> {
    return new Promise((resolve, reject) => {
      const handler = (msg: IpcMessage) => {
        if (msg.type !== 'signal' || msg.method !== 'ready') return;
        clearTimeout(timer);
        this.off('message', handler);
        resolve();
      };

      const timer = setTimeout(() => {
        this.off('message', handler);
        reject(new Error('Plugin did not send ready signal within timeout'));
      }, timeoutMs);

      this.on('message', handler);
    });
  }

  startHeartbeat(): void {
    this.heartbeat.start();
  }

  stopHeartbeat(): void {
    this.heartbeat.stop();
  }

  request(method: string, payload: Record<string, unknown> | null = null): Promise<IpcMessage> {
    return this.tracker.request({
      type: 'request',
      id: randomUUID(),
      method,
      payload,
      error: null,
    });
  }

  sendSignal(method: string, payload: Record<string, unknown> | null = null, id?: string): void {
    this.writeMessage({
      type: 'signal',
      id: id ?? randomUUID(),
      method,
      payload,
      error: null,
    });
  }

  cancel(requestId: string): void {
    this.sendSignal('cancel', { requestId }, requestId);
  }

  sendEvent(method: string, payload: Record<string, unknown> | null = null): void {
    this.writeMessage({
      type: 'event',
      id: randomUUID(),
      method,
      payload,
      error: null,
    });
  }

  async shutdown(timeoutMs = 5000): Promise<void> {
    if (this.closed) return;
    this.closed = true;
    this.stopHeartbeat();

    if (this.proc && this.proc.exitCode === null) {
      try {
        this.sendSignal('deactivate');
      } catch {
        // process may already be dead
      }

      await new Promise<void>((resolve) => {
        const timer = setTimeout(() => {
          this.proc?.kill('SIGKILL');
          resolve();
        }, timeoutMs);

        this.proc!.on('exit', () => {
          clearTimeout(timer);
          resolve();
        });
      });
    }

    this.tracker.failAll('PLUGIN_SHUTDOWN', 'Host shutting down');
  }

  kill(): void {
    this.closed = true;
    this.stopHeartbeat();
    this.proc?.kill('SIGKILL');
    this.tracker.failAll('PLUGIN_KILLED', 'Plugin process killed');
  }

  get process(): ChildProcess | null {
    return this.proc;
  }

  get inflightCount(): number {
    return this.tracker.inflightCount;
  }

  get queueLength(): number {
    return this.tracker.queueLength;
  }

  private onData(chunk: Buffer): void {
    this.readBuf = Buffer.concat([this.readBuf, chunk]);
    let offset = 0;

    while (offset < this.readBuf.length) {
      const parsed = tryParseFrame(this.readBuf, offset);
      if (!parsed) break;
      offset += parsed.bytesConsumed;
      if (parsed.message) {
        this.handleMessage(parsed.message);
      }
    }

    if (offset > 0) {
      this.readBuf = this.readBuf.subarray(offset);
    }
  }

  private handleMessage(msg: IpcMessage): void {
    if (msg.type === 'signal' && msg.method === 'heartbeat-ack') {
      this.heartbeat.acknowledge();
      this.emit('heartbeat-ack');
      return;
    }

    if (msg.type === 'response' && this.tracker.settle(msg)) {
      return;
    }

    this.emit('message', msg);
  }

  private writeMessage(msg: IpcMessage): void {
    if (!this.proc?.stdin?.writable) return;
    this.proc.stdin.write(frameMessage(msg));
  }

  private onProcessExit(code: number | null, signal: string | null): void {
    this.stopHeartbeat();
    this.tracker.failAll(
      'PLUGIN_CRASHED',
      `Plugin process exited (code=${code}, signal=${signal})`,
    );
    this.emit('exit', code, signal);
  }
}
