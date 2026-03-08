import { describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/dev.js';
import {
  createPidFilePath,
  DEFAULT_STARTUP_TIMEOUT_MS,
  fixtureWindowConfig,
  HOST_FIXTURE_PATH,
  waitForPidFile,
  waitForProcessExit,
  withTimeout,
} from './native-host-lifecycle.shared.js';

describe('native host lifecycle (process-level): runtime', () => {
  it('rejects runtime when heartbeat pong is missing', async () => {
    const pidFilePath = createPidFilePath();
    const runtime = await __testOnly.startOutOfProcessRuntime(
      fixtureWindowConfig(),
      {
        hostPath: HOST_FIXTURE_PATH,
        startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
        heartbeatIntervalMs: 100,
        heartbeatTimeoutMs: 150,
        pipeOutput: false,
        env: {
          VOLT_TEST_HOST_SCENARIO: 'no-pong',
          VOLT_TEST_HOST_PID_FILE: pidFilePath,
        },
      },
    );

    const pid = await waitForPidFile(pidFilePath);
    await expect(runtime.run()).rejects.toThrow('Native host heartbeat timed out after 150ms');
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('rejects runtime when host crashes after ready', async () => {
    const pidFilePath = createPidFilePath();
    const runtime = await __testOnly.startOutOfProcessRuntime(
      fixtureWindowConfig(),
      {
        hostPath: HOST_FIXTURE_PATH,
        startupTimeoutMs: 3000,
        pipeOutput: false,
        env: {
          VOLT_TEST_HOST_SCENARIO: 'crash-after-ready',
          VOLT_TEST_HOST_PID_FILE: pidFilePath,
        },
      },
    );

    const pid = await waitForPidFile(pidFilePath);
    await expect(runtime.run()).rejects.toThrow('Native host exited unexpectedly (code=19');
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('rejects runtime when host disconnects IPC channel after ready', async () => {
    const pidFilePath = createPidFilePath();
    const runtime = await __testOnly.startOutOfProcessRuntime(
      fixtureWindowConfig(),
      {
        hostPath: HOST_FIXTURE_PATH,
        startupTimeoutMs: 3000,
        pipeOutput: false,
        env: {
          VOLT_TEST_HOST_SCENARIO: 'disconnect-after-ready',
          VOLT_TEST_HOST_PID_FILE: pidFilePath,
        },
      },
    );

    const pid = await waitForPidFile(pidFilePath);
    await expect(runtime.run()).rejects.toThrow(/Native host IPC channel (disconnected unexpectedly|closed during heartbeat)/);
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('rejects runtime when heartbeat response is delayed beyond timeout', async () => {
    const pidFilePath = createPidFilePath();
    const runtime = await __testOnly.startOutOfProcessRuntime(
      fixtureWindowConfig(),
      {
        hostPath: HOST_FIXTURE_PATH,
        startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
        heartbeatIntervalMs: 100,
        heartbeatTimeoutMs: 150,
        pipeOutput: false,
        env: {
          VOLT_TEST_HOST_SCENARIO: 'delayed-pong',
          VOLT_TEST_HOST_PONG_DELAY_MS: '500',
          VOLT_TEST_HOST_PID_FILE: pidFilePath,
        },
      },
    );

    const pid = await waitForPidFile(pidFilePath);
    await expect(runtime.run()).rejects.toThrow('Native host heartbeat timed out after 150ms');
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('handles window-closed then quit without hanging cleanup', async () => {
    const pidFilePath = createPidFilePath();
    const runtime = await __testOnly.startOutOfProcessRuntime(
      fixtureWindowConfig(),
      {
        hostPath: HOST_FIXTURE_PATH,
        startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
        pipeOutput: false,
        env: {
          VOLT_TEST_HOST_SCENARIO: 'window-closed-then-quit',
          VOLT_TEST_HOST_PID_FILE: pidFilePath,
        },
      },
    );

    const seenEventTypes: string[] = [];
    let cleanupStarted = false;
    let resolveCleanup!: () => void;
    let rejectCleanup!: (reason?: unknown) => void;
    const cleanupDone = new Promise<void>((resolve, reject) => {
      resolveCleanup = resolve;
      rejectCleanup = reject;
    });

    runtime.onEvent((eventJson: string) => {
      try {
        const parsed = __testOnly.parseNativeEvent(JSON.parse(eventJson));
        if (!parsed) {
          return;
        }

        seenEventTypes.push(parsed.type);
        if (parsed.type === 'window-closed') {
          __testOnly.handleWindowClosedEventForIpcState(parsed);
          return;
        }

        if (parsed.type === 'quit' && !cleanupStarted) {
          cleanupStarted = true;
          void (async () => {
            try {
              runtime.shutdown();
              await runtime.run();
              resolveCleanup();
            } catch (error) {
              rejectCleanup(error);
            }
          })();
        }
      } catch (error) {
        rejectCleanup(error);
      }
    });

    await expect(withTimeout(cleanupDone, 5000, 'cleanup handshake')).resolves.toBeUndefined();
    expect(seenEventTypes.slice(0, 2)).toEqual(['window-closed', 'quit']);

    const pid = await waitForPidFile(pidFilePath);
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });
});
