import type { IpcMessage } from './types.js';

interface PendingRequest {
  resolve: (msg: IpcMessage) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

interface QueuedRequest {
  msg: IpcMessage;
  resolve: (msg: IpcMessage) => void;
  reject: (error: Error) => void;
}

export class RequestTracker {
  private readonly pending = new Map<string, PendingRequest>();
  private inflight = 0;
  private queue: QueuedRequest[] = [];

  constructor(
    private readonly callTimeoutMs: number,
    private readonly maxInflight: number,
    private readonly maxQueueDepth: number,
    private readonly writeMessage: (msg: IpcMessage) => void,
  ) {}

  request(msg: IpcMessage): Promise<IpcMessage> {
    return new Promise((resolve, reject) => {
      if (this.inflight < this.maxInflight) {
        this.sendTracked(msg, resolve, reject);
        return;
      }

      if (this.queue.length >= this.maxQueueDepth) {
        this.queue.shift()?.reject(new Error('BACKPRESSURE'));
      }
      this.queue.push({ msg, resolve, reject });
    });
  }

  settle(msg: IpcMessage): boolean {
    const pending = this.pending.get(msg.id);
    if (!pending) return false;

    clearTimeout(pending.timer);
    this.pending.delete(msg.id);
    this.inflight--;
    pending.resolve(msg);
    this.drainQueue();
    return true;
  }

  failAll(code: string, message: string): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(new Error(`${code}: ${message}`));
    }
    this.pending.clear();
    this.inflight = 0;

    for (const queued of this.queue) {
      queued.reject(new Error(`${code}: ${message}`));
    }
    this.queue = [];
  }

  get inflightCount(): number {
    return this.inflight;
  }

  get queueLength(): number {
    return this.queue.length;
  }

  private sendTracked(
    msg: IpcMessage,
    resolve: (msg: IpcMessage) => void,
    reject: (error: Error) => void,
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
      const next = this.queue.shift();
      if (!next) return;
      this.sendTracked(next.msg, next.resolve, next.reject);
    }
  }
}
