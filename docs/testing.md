# Testing

Volt includes a first-party E2E package (`@voltkit/volt-test`) and CLI entrypoint:

```bash
volt test
```

## Configuration

Create `volt.test.config.ts` at your project root:

```ts
import {
  createHelloWorldSmokeSuite,
  createIpcDemoSmokeSuite,
  defineTestConfig,
} from '@voltkit/volt-test';

export default defineTestConfig({
  timeoutMs: 120_000,
  retries: 1,
  artifactDir: 'artifacts/e2e-local',
  suites: [
    createHelloWorldSmokeSuite({ projectDir: 'examples/hello-world' }),
    createIpcDemoSmokeSuite({ projectDir: 'examples/ipc-demo' }),
  ],
});
```

## Built-In Smoke Suites

- `hello-world-smoke`: builds/launches `examples/hello-world`, validates ping + heading, captures artifacts.
- `ipc-demo-smoke`: builds/launches `examples/ipc-demo`, validates ping/echo/compute/native setup/db flow, and enforces deterministic window-ready assertions.

## Artifacts, Retries, and Flake Capture

`volt test` now captures per-attempt artifacts by default:

- `app-process.log`
- `result-payload.json`
- `run-summary.json`
- `flake-report.json` (only when a suite passes after retry)
- best-effort screenshots (`*.png`)

Artifact layout:

```text
artifacts/volt-test/<timestamp>/
  <suite-name>/
    attempt-1/
    attempt-2/
  run-summary.json
  flake-report.json
```

## Automation Drivers

`@voltkit/volt-test` exports:

- `MenuAutomationDriver`
- `TrayAutomationDriver`
- `FileDialogAutomationDriver` (platform-aware open/save payload normalization and assertions)

## Window Wait/Assertion APIs

Use deterministic window checks in custom suites:

```ts
import { assertWindowReady, parseWindowStatus } from '@voltkit/volt-test';

const parsed = parseWindowStatus(statusPayload);
assertWindowReady(parsed, 1);
```

For polling flows:

```ts
import { waitForWindowStatus } from '@voltkit/volt-test';

await waitForWindowStatus(readStatus, (status) => status.nativeReady && status.windowCount >= 1, {
  timeoutMs: 12_000,
  intervalMs: 150,
  description: 'native ready status',
});
```

## Authoring Templates

Native E2E authoring templates are available in:

- `docs/templates/native-e2e-suite.template.ts`
- `docs/templates/native-e2e-backend.template.ts`
- `docs/templates/native-e2e-frontend.template.ts`
