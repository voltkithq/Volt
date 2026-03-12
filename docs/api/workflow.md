# Workflow

Native-backed workflow helpers for batch pipeline execution.

No additional permission is required.

Use these APIs from the renderer through `voltkit/renderer`:

```ts
import { workflow } from 'voltkit/renderer';
```

Volt routes these calls through reserved `volt:native:*` IPC channels and executes the heavy pipeline work in Rust.

## `workflow.listPlugins(): Promise<WorkflowPluginInfo[]>`

Return the public plugin metadata exposed by the workflow example/runtime.

```ts
const plugins = await workflow.listPlugins();
console.log(plugins.map((plugin) => plugin.name));
```

### `WorkflowPluginInfo`

```ts
interface WorkflowPluginInfo {
  name: string;
  label: string;
  description: string;
}
```

## `workflow.run(options?): Promise<WorkflowRunResult>`

Run a native workflow pipeline against a synthetic document batch.

```ts
const result = await workflow.run({
  batchSize: 3_000,
  passes: 3,
  pipeline: ['normalizeText', 'extractSignals', 'buildDigests'],
});

console.log(result.backendDurationMs);
console.log(result.routeDistribution);
```

### `WorkflowRunOptions`

```ts
interface WorkflowRunOptions {
  batchSize?: number;
  passes?: number;
  pipeline?: string[];
}
```

### `WorkflowRunResult`

```ts
interface WorkflowRunResult {
  batchSize: number;
  passes: number;
  pipeline: string[];
  backendDurationMs: number;
  stepTimings: Array<{ plugin: string; durationMs: number }>;
  routeDistribution: Record<string, number>;
  averagePriority: number;
  digestSample: string[];
  payloadBytes: number;
}
```

## Notes

- `workflow.listPlugins()` is a regular public IPC helper. It is safe to call from the renderer and avoids hard-coding raw channel names in app code.
- `workflow.run()` is intended for whole-pipeline execution, not per-record IPC chatter.
- The underlying `volt:native:*` channel names are reserved implementation details; app code should call `workflow.run()` instead of invoking those channels directly.
