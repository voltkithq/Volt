import {
  createHelloWorldSmokeSuite,
  createIpcDemoSmokeSuite,
  defineTestConfig,
} from '@voltkit/volt-test';

export default defineTestConfig({
  timeoutMs: 600_000,
  retries: 1,
  artifactDir: 'artifacts/e2e-smoke/default',
  suites: [
    createHelloWorldSmokeSuite({
      projectDir: 'examples/hello-world',
      timeoutMs: 600_000,
    }),
    createIpcDemoSmokeSuite({
      projectDir: 'examples/ipc-demo',
      timeoutMs: 600_000,
    }),
  ],
});
