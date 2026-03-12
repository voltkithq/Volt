import {
  createAnalyticsStudioBenchmarkSuite,
  createSyncStormBenchmarkSuite,
  createWorkflowLabBenchmarkSuite,
  defineTestConfig,
} from '@voltkit/volt-test';

export default defineTestConfig({
  timeoutMs: 900_000,
  retries: 0,
  artifactDir: 'artifacts/volt-benchmarks/default',
  suites: [
    createAnalyticsStudioBenchmarkSuite({
      projectDir: 'examples/analytics-studio',
      timeoutMs: 900_000,
    }),
    createSyncStormBenchmarkSuite({
      projectDir: 'examples/sync-storm',
      timeoutMs: 900_000,
    }),
    createWorkflowLabBenchmarkSuite({
      projectDir: 'examples/workflow-lab',
      timeoutMs: 900_000,
    }),
  ],
});
