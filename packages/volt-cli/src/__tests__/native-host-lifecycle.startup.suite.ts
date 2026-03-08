import { describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/dev.js';
import {
  createPidFilePath,
  DEFAULT_RUNTIME_SHUTDOWN_TIMEOUT_MS,
  DEFAULT_STARTUP_TIMEOUT_MS,
  fixtureWindowConfig,
  HOST_FIXTURE_PATH,
  trackSpawnedPid,
  waitForPidFile,
  waitForProcessExit,
  withTimeout,
} from './native-host-lifecycle.shared.js';

describe('native host lifecycle (process-level): startup', () => {
  it('starts with ready signal and shuts down cleanly', async () => {
    const runtime = await __testOnly.startOutOfProcessRuntime(
      fixtureWindowConfig(),
      {
        hostPath: HOST_FIXTURE_PATH,
        startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
        pipeOutput: false,
        env: {
          VOLT_TEST_HOST_SCENARIO: 'ready',
        },
      },
    );

    runtime.shutdown();
    await expect(withTimeout(runtime.run(), DEFAULT_RUNTIME_SHUTDOWN_TIMEOUT_MS, 'runtime shutdown')).resolves.toBeUndefined();
  });

  it('fails startup on runtime-error and cleans host process', async () => {
    const pidFilePath = createPidFilePath();
    await expect(
      __testOnly.startOutOfProcessRuntime(
        fixtureWindowConfig(),
        {
          hostPath: HOST_FIXTURE_PATH,
          startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
          pipeOutput: false,
          env: {
            VOLT_TEST_HOST_SCENARIO: 'runtime-error-hang',
            VOLT_TEST_HOST_PID_FILE: pidFilePath,
          },
        },
      ),
    ).rejects.toThrow('fake runtime failure');

    const pid = await waitForPidFile(pidFilePath);
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('fails startup on native-unavailable and cleans host process', async () => {
    const pidFilePath = createPidFilePath();
    await expect(
      __testOnly.startOutOfProcessRuntime(
        fixtureWindowConfig(),
        {
          hostPath: HOST_FIXTURE_PATH,
          startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
          pipeOutput: false,
          env: {
            VOLT_TEST_HOST_SCENARIO: 'native-unavailable',
            VOLT_TEST_HOST_PID_FILE: pidFilePath,
          },
        },
      ),
    ).rejects.toThrow('fake native unavailable');

    const pid = await waitForPidFile(pidFilePath);
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('fails startup when host exits before ready', async () => {
    await expect(
      __testOnly.startOutOfProcessRuntime(
        fixtureWindowConfig(),
        {
          hostPath: HOST_FIXTURE_PATH,
          startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
          pipeOutput: false,
          env: {
            VOLT_TEST_HOST_SCENARIO: 'early-exit',
          },
        },
      ),
    ).rejects.toThrow('Native host exited before ready (code=23)');
  });

  it('fails startup on timeout and cleans host process', async () => {
    const pidFilePath = createPidFilePath();
    let spawnedPid: number | null = null;

    await expect(
      __testOnly.startOutOfProcessRuntime(
        fixtureWindowConfig(),
        {
          hostPath: HOST_FIXTURE_PATH,
          startupTimeoutMs: 200,
          pipeOutput: false,
          onSpawn: (pid) => {
            spawnedPid = pid;
            trackSpawnedPid(pid);
          },
          env: {
            VOLT_TEST_HOST_SCENARIO: 'silent-hang',
            VOLT_TEST_HOST_PID_FILE: pidFilePath,
          },
        },
      ),
    ).rejects.toThrow('Native host startup timed out after 200ms');

    let pid: number;
    try {
      pid = await waitForPidFile(pidFilePath, 3000);
    } catch (error) {
      if (spawnedPid && Number.isInteger(spawnedPid)) {
        pid = spawnedPid;
      } else {
        throw error;
      }
    }
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });

  it('rejects unsupported host protocol version and cleans host process', async () => {
    const pidFilePath = createPidFilePath();
    await expect(
      __testOnly.startOutOfProcessRuntime(
        fixtureWindowConfig(),
        {
          hostPath: HOST_FIXTURE_PATH,
          startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
          pipeOutput: false,
          env: {
            VOLT_TEST_HOST_SCENARIO: 'wrong-version-ready',
            VOLT_TEST_HOST_PID_FILE: pidFilePath,
          },
        },
      ),
    ).rejects.toThrow('Unsupported host protocol version');

    const pid = await waitForPidFile(pidFilePath);
    await expect(waitForProcessExit(pid)).resolves.toBe(true);
  });
});
