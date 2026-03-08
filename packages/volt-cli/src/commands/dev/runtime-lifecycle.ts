import { fork } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import type { NativeHostWindowConfig } from '../native-host-protocol.js';
import type {
  NativeRuntimeBridge,
  OutOfProcessRuntimeOptions,
} from './types.js';
import { createNativeHostHeartbeatController } from './runtime-heartbeat.js';
import { routeNativeHostMessage } from './runtime-lifecycle-message-routing.js';
import { killChildAndWait, scheduleForcedChildKill } from './runtime-lifecycle-shutdown.js';
import { createNativeHostStartupHandshake } from './runtime-lifecycle-startup-handshake.js';
import { createHostProcessMessenger } from './runtime-process-messaging.js';
import {
  DEFAULT_NATIVE_HOST_HEARTBEAT_INTERVAL_MS,
  DEFAULT_NATIVE_HOST_HEARTBEAT_TIMEOUT_MS,
  DEFAULT_NATIVE_HOST_STARTUP_TIMEOUT_MS,
} from './runtime-protocol.js';

export { startInProcessRuntime } from './runtime-lifecycle-in-process.js';

export async function startOutOfProcessRuntime(
  windowConfig: NativeHostWindowConfig,
  options: OutOfProcessRuntimeOptions = {},
): Promise<NativeRuntimeBridge> {
  const hostPath = options.hostPath ?? fileURLToPath(new URL('./native-host.js', import.meta.url));
  const child = fork(hostPath, [], {
    stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
    env: options.env ? { ...process.env, ...options.env } : process.env,
  });
  if (typeof child.pid === 'number' && Number.isInteger(child.pid)) {
    options.onSpawn?.(child.pid);
  }

  if (options.pipeOutput ?? true) {
    child.stdout?.on('data', (chunk) => process.stdout.write(chunk));
    child.stderr?.on('data', (chunk) => process.stderr.write(chunk));
  }

  const messenger = createHostProcessMessenger(child);
  let eventListener: (eventJson: string) => void = () => {};
  let sawStarting = false;
  let sawStopping = false;
  let sawStopped = false;
  let shutdownRequested = false;
  let runtimeFailure: Error | null = null;
  const startupHandshake = createNativeHostStartupHandshake();
  const startupTimeoutMs = options.startupTimeoutMs ?? DEFAULT_NATIVE_HOST_STARTUP_TIMEOUT_MS;
  const heartbeatIntervalMs =
    options.heartbeatIntervalMs ?? DEFAULT_NATIVE_HOST_HEARTBEAT_INTERVAL_MS;
  const heartbeatTimeoutMs = options.heartbeatTimeoutMs ?? DEFAULT_NATIVE_HOST_HEARTBEAT_TIMEOUT_MS;
  const disconnectGraceMs = Math.max(heartbeatTimeoutMs, 500);
  const failProtocol = (message: string) => {
    const error = new Error(message);
    runtimeFailure = runtimeFailure ?? error;
    if (!startupHandshake.isReadySettled()) {
      startupHandshake.rejectReady(error);
    } else {
      console.error(`[volt] ${message}`);
    }
    heartbeat.stop();
    if (!child.killed) {
      child.kill();
    }
  };
  const heartbeat = createNativeHostHeartbeatController({
    child,
    heartbeatIntervalMs,
    heartbeatTimeoutMs,
    onProtocolFailure: failProtocol,
  });
  const startupTimeout = setTimeout(() => {
    startupHandshake.rejectReady(
      new Error(`Native host startup timed out after ${startupTimeoutMs}ms`),
    );
  }, startupTimeoutMs);
  startupTimeout.unref();

  const exited = new Promise<void>((resolve, reject) => {
    child.on('exit', (code, signal) => {
      heartbeat.stop();
      if (runtimeFailure) {
        reject(runtimeFailure);
        return;
      }

      const expectedExit = shutdownRequested || sawStopping || sawStopped || code === 0;
      if (expectedExit) {
        resolve();
        return;
      }

      reject(
        new Error(`Native host exited unexpectedly (code=${code ?? 'null'}, signal=${signal ?? 'null'})`),
      );
    });
    child.on('error', reject);
  });

  child.on('exit', (code) => {
    if (!startupHandshake.isReadySettled()) {
      startupHandshake.rejectReady(
        new Error(`Native host exited before ready (code=${code ?? 'null'})`),
      );
    }
    heartbeat.stop();
  });

  child.on('error', (error) => {
    if (!startupHandshake.isReadySettled()) {
      startupHandshake.rejectReady(error);
    }
  });
  child.on('disconnect', () => {
    if (
      !startupHandshake.isReadySettled() ||
      shutdownRequested ||
      sawStopping ||
      sawStopped ||
      runtimeFailure
    ) {
      return;
    }
    const disconnectCheckTimer = setTimeout(() => {
      if (shutdownRequested || sawStopping || sawStopped || runtimeFailure) {
        return;
      }
      if (child.exitCode !== null || child.signalCode !== null) {
        return;
      }
      failProtocol('Native host IPC channel disconnected unexpectedly');
    }, disconnectGraceMs);
    disconnectCheckTimer.unref();
  });

  child.on('message', (raw) => {
    routeNativeHostMessage(raw, {
      onStarting(): void {
        sawStarting = true;
      },
      onReady(): void {
        if (!sawStarting) {
          failProtocol('Host protocol violation: received ready before starting');
          return;
        }
        startupHandshake.resolveReady();
        heartbeat.start();
      },
      onEvent(eventJson: string): void {
        eventListener(eventJson);
      },
      onRuntimeError(message: string): void {
        if (!startupHandshake.isReadySettled()) {
          startupHandshake.rejectReady(new Error(message));
        } else {
          failProtocol(`Native host runtime error: ${message}`);
        }
      },
      onNativeUnavailable(message: string): void {
        if (!startupHandshake.isReadySettled()) {
          startupHandshake.rejectReady(new Error(message));
        } else {
          failProtocol(`Native host unavailable after startup: ${message}`);
        }
      },
      onPong(pingId: number): void {
        heartbeat.handlePong(pingId);
      },
      onStopping(): void {
        sawStopping = true;
      },
      onStopped(): void {
        sawStopped = true;
        heartbeat.stop();
      },
      onProtocolFailure(message: string): void {
        failProtocol(message);
      },
    });
  });

  if (!messenger.sendStart(windowConfig)) {
    clearTimeout(startupTimeout);
    heartbeat.stop();
    await killChildAndWait(child, exited, 1000);
    throw new Error('Native host IPC channel was not available for startup');
  }

  try {
    await startupHandshake.ready;
  } catch (err) {
    clearTimeout(startupTimeout);
    heartbeat.stop();
    await killChildAndWait(child, exited, 1000);
    void exited.catch(() => {});
    throw err;
  }
  clearTimeout(startupTimeout);

  return {
    onEvent(callback: (eventJson: string) => void): void {
      eventListener = callback;
    },
    windowEvalScript(jsId: string, script: string): void {
      messenger.sendEvalScript(jsId, script);
    },
    windowClose(jsId: string): void {
      messenger.sendWindowCommand('close-window', jsId);
    },
    windowShow(jsId: string): void {
      messenger.sendWindowCommand('show-window', jsId);
    },
    windowFocus(jsId: string): void {
      messenger.sendWindowCommand('focus-window', jsId);
    },
    windowMaximize(jsId: string): void {
      messenger.sendWindowCommand('maximize-window', jsId);
    },
    windowMinimize(jsId: string): void {
      messenger.sendWindowCommand('minimize-window', jsId);
    },
    windowRestore(jsId: string): void {
      messenger.sendWindowCommand('restore-window', jsId);
    },
    run(): Promise<void> {
      return exited;
    },
    shutdown(): void {
      shutdownRequested = true;
      heartbeat.stop();
      messenger.sendShutdown();
      scheduleForcedChildKill(child, 2000);
    },
  };
}
