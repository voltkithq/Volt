import test from 'node:test';
import assert from 'node:assert/strict';

import { soakTasks } from './soak-config.mjs';

test('linux soak uses the headless-safe bridge regression command', () => {
  const task = soakTasks('linux').find((entry) => entry.name === 'rust-command-bridge-lifecycle');
  assert.ok(task, 'expected lifecycle soak task');
  assert.deepEqual(task.args, [
    'test',
    '-p',
    'volt-core',
    'command::tests::command_metrics_track_failed_send',
  ]);
});

test('windows soak keeps the full lifecycle bridge regression command', () => {
  const task = soakTasks('win32').find((entry) => entry.name === 'rust-command-bridge-lifecycle');
  assert.ok(task, 'expected lifecycle soak task');
  assert.deepEqual(task.args, [
    'test',
    '-p',
    'volt-core',
    'command::tests::lifecycle_drop_clears_bridge',
  ]);
});
