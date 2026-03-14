/**
 * Plugin-Host IPC Protocol — host side.
 *
 * Framed JSON messages over stdin/stdout with 4-byte LE length prefix.
 * Handles correlation, heartbeat, cancellation, and backpressure.
 */

import { ChildProcess, spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { EventEmitter } from 'node:events';

// ── Wire Types ───────────────────────────────────────────────────

export type MessageType = 'request' | 'response' | 'event' | 'signal';

export interface IpcMessage {
  type: MessageType;
  id: string;
  method: string;
  payload: Record<string, unknown> | null;
  error: { code: string; message: string } | null;
}

// ── Framing ──────────────────────────────────────────────────────

export function frameMessage(msg: IpcMessage): Buffer {
  const json = JSON.stringify(msg);
  const body = Buffer.from(json + '\n', 'utf-8');
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  return Buffer.concat([header, body]);
}

export interface ParsedFrame {
  message: IpcMessage;
  bytesConsumed: number;
}

export function tryParseFrame(buffer: Buffer, offset: number): ParsedFrame | null {
  if (buffer.length - offset < 4) return null;
  const length = buffer.readUInt32LE(offset);
  if (length === 0 || length > 16 * 1024 * 1024) return null;
  if (buffer.length - offset - 4 < length) return null;
  const jsonBytes = buffer.subarray(offset + 4, offset + 4 + length);
  const raw = jsonBytes.toString('utf-8');
  const stripped = raw.endsWith('\n') ? raw.slice(0, -1) : raw;
  const message = JSON.parse(stripped) as IpcMessage;
  return { message, bytesConsumed: 4 + length };
}

// ── Pending Request Tracking ─────────────────────────────────────

interface PendingRequest {
  resolve: (msg: IpcMessage) => void;
  reject: (err: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

// ── Options ──────────────────────────────────────────────────────

export interface PluginHostOptions {
  heartbeatIntervalMs?: number;
  heartbeatTimeoutMs?: number;
  callTimeoutMs?: number;
  maxInflight?: number;
  maxQueueDepth?: number;
}

const DEFAULTS = {
  heartbeatIntervalMs: 5000,
  heartbeatTimeoutMs: 3000,
  callTimeoutMs: 30000,
  maxInflight: 64,
  maxQueueDepth: 256,
} as const;

// ── Host Connection ──────────────────────────────────────────────

export class PluginIpcHost extends EventEmitter {
  private proc: ChildProcess | null = null;
  private readBuf = Buffer.alloc(0);
  private pending = new Map<string, PendingRequest>();
  private inflight = 0;
  private queue: Array<{ msg: IpcMessage; resolve: (m: IpcMessage) => void; reject: (e: Error) => void }> = [];
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private missedHeartbeats = 0;
  private awaitingHeartbeatAck = false;
  private closed = false;

  private readonly heartbeatIntervalMs: number;
  private readonly heartbeatTimeoutMs: number;
  private readonly callTimeoutMs: number;
  private readonly maxInflight: number;
  private readonly maxQueueDepth: number;

  constructor(opts?: PluginHostOptions) {
    super();
    this.heartbeatIntervalMs = opts?.heartbeatIntervalMs ?? DEFAULTS.heartbeatIntervalMs;
    this.heartbeatTimeoutMs = opts?.heartbeatTimeoutMs ?? DEFAULTS.heartbeatTimeoutMs;
    this.callTimeoutMs = opts?.callTimeoutMs ?? DEFAULTS.callTimeoutMs;
    this.maxInflight = opts?.maxInflight ?? DEFAULTS.maxInflight;
    this.maxQueueDepth = opts?.maxQueueDepth ?? DEFAULTS.maxQueueDepth;
  }

  /**
   * Spawn a plugin host process and wire up stdin/stdout IPC.
   */
  spawn(command: string, args: string[]): void {
    if (this.proc) throw new Error('Already spawned');
    this.proc = spawn(command, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    this.proc.stdout!.on('data', (chunk: Buffer) => this.onData(chunk));
    this.proc.stderr!.on('data', (chunk: Buffer) => this.emit('stderr', chunk.toString('utf-8')));
    this.proc.on('exit', (code, signal) => {
      this.onProcessExit(code, signal);
    });
  }

  /**
   * Attach to an already-spawned process (for testing).
   */
  attach(proc: ChildProcess): void {
    if (this.proc) throw new Error('Already spawned');
    this.proc = proc;
    this.proc.stdout!.on('data', (chunk: Buffer) => this.onData(chunk));
    this.proc.stderr!.on('data', (chunk: Buffer) => this.emit('stderr', chunk.toString('utf-8')));
    this.proc.on('exit', (code, signal) => {
      this.onProcessExit(code, signal);
    });
  }

  /**
   * Wait for the plugin to send the "ready" signal.
   */
  waitForReady(timeoutMs = 10000): Promise<void> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new Error('Plugin did not send ready signal within timeout'));
      }, timeoutMs);

      const handler = (msg: IpcMessage) => {
        if (msg.type === 'signal' && msg.method === 'ready') {
          clearTimeout(timer);
          this.off('message', handler);
          resolve();
        }
      };
      this.on('message', handler);
    });
  }

  /**
   * Start sending heartbeats.
   */
  startHeartbeat(): void {
    if (this.heartbeatTimer) return;
    this.missedHeartbeats = 0;
    this.heartbeatTimer = setInterval(() => this.sendHeartbeat(), this.heartbeatIntervalMs);
  }

  /**
   * Stop sending heartbeats.
   */
  stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * Send a request and wait for a correlated response.
   */
  request(method: string, payload: Record<string, unknown> | null = null): Promise<IpcMessage> {
    const id = randomUUID();
    const msg: IpcMessage = { type: 'request', id, method, payload, error: null };
    return this.enqueueOrSend(msg);
  }

  /**
   * Send a signal (fire-and-forget from the host's perspective).
   */
  sendSignal(method: string, payload: Record<string, unknown> | null = null, id?: string): void {
    const msg: IpcMessage = {
      type: 'signal',
      id: id ?? randomUUID(),
      method,
      payload,
      error: null,
    };
    this.writeMessage(msg);
  }

  /**
   * Send a cancellation signal for a pending request.
   */
  cancel(requestId: string): void {
    this.sendSignal('cancel', { requestId }, requestId);
  }

  /**
   * Send an event (fire-and-forget).
   */
  sendEvent(method: string, payload: Record<string, unknown> | null = null): void {
    const msg: IpcMessage = {
      type: 'event',
      id: randomUUID(),
      method,
      payload,
      error: null,
    };
    this.writeMessage(msg);
  }

  /**
   * Gracefully shut down — send deactivate, stop heartbeat, kill process.
   */
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

    this.failAllPending('PLUGIN_SHUTDOWN', 'Host shutting down');
  }

  /**
   * Kill the process immediately.
   */
  kill(): void {
    this.closed = true;
    this.stopHeartbeat();
    this.proc?.kill('SIGKILL');
    this.failAllPending('PLUGIN_KILLED', 'Plugin process killed');
  }

  get process(): ChildProcess | null {
    return this.proc;
  }

  get inflightCount(): number {
    return this.inflight;
  }

  get queueLength(): number {
    return this.queue.length;
  }

  // ── Internal ───────────────────────────────────────────────────

  private onData(chunk: Buffer): void {
    this.readBuf = Buffer.concat([this.readBuf, chunk]);
    let offset = 0;

    while (offset < this.readBuf.length) {
      const parsed = tryParseFrame(this.readBuf, offset);
      if (!parsed) break;
      offset += parsed.bytesConsumed;
      this.handleMessage(parsed.message);
    }

    if (offset > 0) {
      this.readBuf = this.readBuf.subarray(offset);
    }
  }

  private handleMessage(msg: IpcMessage): void {
    // Heartbeat ack
    if (msg.type === 'signal' && msg.method === 'heartbeat-ack') {
      this.awaitingHeartbeatAck = false;
      this.missedHeartbeats = 0;
      this.emit('heartbeat-ack');
      return;
    }

    // Response correlation
    if (msg.type === 'response') {
      const req = this.pending.get(msg.id);
      if (req) {
        clearTimeout(req.timer);
        this.pending.delete(msg.id);
        this.inflight--;
        req.resolve(msg);
        this.drainQueue();
      }
      return;
    }

    this.emit('message', msg);
  }

  private sendHeartbeat(): void {
    if (this.awaitingHeartbeatAck) {
      this.missedHeartbeats++;
      if (this.missedHeartbeats >= 2) {
        this.emit('unresponsive');
        this.kill();
        return;
      }
    }
    this.awaitingHeartbeatAck = true;
    this.sendSignal('heartbeat');

    setTimeout(() => {
      if (this.awaitingHeartbeatAck) {
        // Will be counted as a miss on next heartbeat interval
      }
    }, this.heartbeatTimeoutMs);
  }

  private enqueueOrSend(msg: IpcMessage): Promise<IpcMessage> {
    return new Promise((resolve, reject) => {
      if (this.inflight < this.maxInflight) {
        this.sendTracked(msg, resolve, reject);
      } else {
        if (this.queue.length >= this.maxQueueDepth) {
          const dropped = this.queue.shift()!;
          dropped.reject(new Error('BACKPRESSURE'));
        }
        this.queue.push({ msg, resolve, reject });
      }
    });
  }

  private sendTracked(
    msg: IpcMessage,
    resolve: (m: IpcMessage) => void,
    reject: (e: Error) => void,
  ): void {
    this.inflight++;
    const timer = setTimeout(() => {
      this.pending.delete(msg.id);
      this.inflight--;
      reject(new Error(`Request ${msg.method} timed out after ${this.callTimeoutMs}ms`));
      this.drainQueue();
    }, this.callTimeoutMs);

    this.pending.set(msg.id, { resolve, reject, timer });
    this.writeMessage(msg);
  }

  private drainQueue(): void {
    while (this.queue.length > 0 && this.inflight < this.maxInflight) {
      const next = this.queue.shift()!;
      this.sendTracked(next.msg, next.resolve, next.reject);
    }
  }

  private writeMessage(msg: IpcMessage): void {
    if (!this.proc?.stdin?.writable) return;
    const frame = frameMessage(msg);
    this.proc.stdin.write(frame);
  }

  private onProcessExit(code: number | null, signal: string | null): void {
    this.stopHeartbeat();
    this.failAllPending('PLUGIN_CRASHED', `Plugin process exited (code=${code}, signal=${signal})`);
    this.emit('exit', code, signal);
  }

  private failAllPending(code: string, message: string): void {
    for (const [id, req] of this.pending) {
      clearTimeout(req.timer);
      req.reject(new Error(`${code}: ${message}`));
    }
    this.pending.clear();
    this.inflight = 0;

    for (const queued of this.queue) {
      queued.reject(new Error(`${code}: ${message}`));
    }
    this.queue = [];
  }
}
