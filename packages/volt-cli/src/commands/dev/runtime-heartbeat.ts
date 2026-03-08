import type { ChildProcess } from 'node:child_process';
import { createHostProcessMessenger } from './runtime-process-messaging.js';

export interface NativeHostHeartbeatController {
  start(): void;
  stop(): void;
  handlePong(pingId: number): void;
}

export interface NativeHostHeartbeatControllerOptions {
  child: ChildProcess;
  heartbeatIntervalMs: number;
  heartbeatTimeoutMs: number;
  onProtocolFailure: (message: string) => void;
}

export function createNativeHostHeartbeatController(
  options: NativeHostHeartbeatControllerOptions,
): NativeHostHeartbeatController {
  const { child, heartbeatIntervalMs, heartbeatTimeoutMs, onProtocolFailure } = options;
  const messenger = createHostProcessMessenger(child);
  let nextPingId = 1;
  let pendingPingId: number | null = null;
  let heartbeatInterval: NodeJS.Timeout | null = null;
  let heartbeatTimeout: NodeJS.Timeout | null = null;

  const stop = () => {
    if (heartbeatInterval) {
      clearInterval(heartbeatInterval);
      heartbeatInterval = null;
    }
    if (heartbeatTimeout) {
      clearTimeout(heartbeatTimeout);
      heartbeatTimeout = null;
    }
    pendingPingId = null;
  };

  const fail = (message: string) => {
    stop();
    onProtocolFailure(message);
  };

  const armHeartbeatTimeout = () => {
    if (heartbeatTimeout) {
      clearTimeout(heartbeatTimeout);
    }
    heartbeatTimeout = setTimeout(() => {
      fail(`Native host heartbeat timed out after ${heartbeatTimeoutMs}ms`);
    }, heartbeatTimeoutMs);
    heartbeatTimeout.unref();
  };

  return {
    start(): void {
      if (heartbeatIntervalMs <= 0 || heartbeatTimeoutMs <= 0 || heartbeatInterval) {
        return;
      }

      heartbeatInterval = setInterval(() => {
        if (pendingPingId !== null) {
          fail(`Native host heartbeat timed out after ${heartbeatTimeoutMs}ms`);
          return;
        }

        pendingPingId = nextPingId;
        nextPingId += 1;
        if (!messenger.sendPing(pendingPingId)) {
          fail('Native host IPC channel closed during heartbeat');
          return;
        }

        armHeartbeatTimeout();
      }, heartbeatIntervalMs);
      heartbeatInterval.unref();
    },
    stop,
    handlePong(pingId: number): void {
      if (pendingPingId === null || pingId !== pendingPingId) {
        fail(
          `Host protocol violation: unexpected pong id ${pingId} (pending ${String(pendingPingId)})`,
        );
        return;
      }

      pendingPingId = null;
      if (heartbeatTimeout) {
        clearTimeout(heartbeatTimeout);
        heartbeatTimeout = null;
      }
    },
  };
}
