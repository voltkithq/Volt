import {
  createHelloWorldSmokeSuite,
  createIpcDemoSmokeSuite,
  defineTestConfig,
} from '@voltkit/volt-test';

export default defineTestConfig({
  timeoutMs: 120_000,
  retries: 1,
  artifactDir: 'artifacts/e2e-smoke/default',
  suites: [
    createHelloWorldSmokeSuite({
      projectDir: 'examples/hello-world',
    }),
    createIpcDemoSmokeSuite({
      projectDir: 'examples/ipc-demo',
      timeoutMs: 300_000,
    }),
  ],
});
