export const DROPPED_DISPATCH_PATTERN = '[volt] Dropped native event callback dispatches:';
export const DEFAULT_SOAK_ITERATIONS = 10;
export const FAILURE_OUTPUT_TAIL_LINES = 30;

const TASKS = [
  {
    name: 'rust-ipc-integration',
    command: 'cargo',
    args: ['test', '-p', 'volt-core', '--test', 'ipc_integration', 'full_ipc_roundtrip_success'],
    p95MaxMsByPlatform: { linux: 15_000, darwin: 20_000, win32: 30_000 },
  },
  {
    name: 'rust-runtime-startup',
    command: 'cargo',
    args: ['test', '-p', 'volt-napi', 'app::tests::test_wait_for_runtime_start_with_probe_success'],
    p95MaxMsByPlatform: { linux: 20_000, darwin: 25_000, win32: 35_000 },
  },
  {
    name: 'rust-event-contract-fixture',
    command: 'cargo',
    args: ['test', '-p', 'volt-napi', 'app::tests::test_payloads_match_contract_fixture'],
    p95MaxMsByPlatform: { linux: 15_000, darwin: 20_000, win32: 30_000 },
  },
  {
    name: 'rust-command-bridge-lifecycle',
    command: 'cargo',
    args: ['test', '-p', 'volt-core', 'command::tests::lifecycle_drop_clears_bridge'],
    p95MaxMsByPlatform: { linux: 15_000, darwin: 20_000, win32: 30_000 },
  },
  {
    name: 'framework-platform-behavior',
    command: pnpmCommand(),
    args: ['--filter', 'voltkit', 'run', 'test'],
    p95MaxMsByPlatform: { linux: 15_000, darwin: 18_000, win32: 25_000 },
  },
  {
    name: 'framework-shortcut-contract',
    command: pnpmCommand(),
    args: ['--filter', 'voltkit', 'exec', 'vitest', 'run', 'src/__tests__/globalShortcut.test.ts'],
    p95MaxMsByPlatform: { linux: 7_000, darwin: 8_000, win32: 10_000 },
  },
  {
    name: 'cli-dev-ipc-bridge',
    command: pnpmCommand(),
    args: ['--filter', 'volt-cli', 'run', 'test'],
    p95MaxMsByPlatform: { linux: 18_000, darwin: 20_000, win32: 30_000 },
  },
  {
    name: 'cli-event-contract-parser',
    command: pnpmCommand(),
    args: ['--filter', 'volt-cli', 'exec', 'vitest', 'run', 'src/__tests__/event-contracts.test.ts'],
    p95MaxMsByPlatform: { linux: 7_000, darwin: 8_000, win32: 10_000 },
  },
  {
    name: 'cli-runtime-mode-contract',
    command: pnpmCommand(),
    args: ['--filter', 'volt-cli', 'exec', 'vitest', 'run', 'src/__tests__/runtime-mode.test.ts'],
    p95MaxMsByPlatform: { linux: 7_000, darwin: 8_000, win32: 10_000 },
  },
];

export function soakTasks() {
  return TASKS.map((task) => ({
    ...task,
    args: [...task.args],
    p95MaxMsByPlatform: { ...task.p95MaxMsByPlatform },
  }));
}

export function loadSoakRuntimeConfig(env = process.env) {
  return {
    iterations: parsePositiveInt(env.SOAK_ITERATIONS, DEFAULT_SOAK_ITERATIONS),
    warmup: parseBoolean(env.SOAK_WARMUP, true),
    metricsFile:
      env.SOAK_METRICS_FILE ?? `artifacts/soak-metrics-${process.platform}-${process.arch}.json`,
    trendFile:
      env.SOAK_TREND_FILE ?? `artifacts/soak-trend-${process.platform}-${process.arch}.jsonl`,
    reportFile:
      env.SOAK_REPORT_FILE ?? `artifacts/soak-report-${process.platform}-${process.arch}.md`,
  };
}

export function thresholdForPlatform(task, platform) {
  if (task.p95MaxMsByPlatform[platform] != null) {
    return task.p95MaxMsByPlatform[platform];
  }
  const values = Object.values(task.p95MaxMsByPlatform);
  return values.length > 0 ? Math.max(...values) : 0;
}

function pnpmCommand() {
  return 'pnpm';
}

function parsePositiveInt(raw, fallback) {
  const parsed = Number.parseInt(raw ?? '', 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return fallback;
  }
  return parsed;
}

function parseBoolean(raw, fallback) {
  if (raw == null || raw.trim() === '') {
    return fallback;
  }
  const normalized = raw.trim().toLowerCase();
  if (normalized === '1' || normalized === 'true' || normalized === 'yes') {
    return true;
  }
  if (normalized === '0' || normalized === 'false' || normalized === 'no') {
    return false;
  }
  return fallback;
}
