# Plugin Startup Benchmark

This benchmark measures the current cold-start cost of the plugin host before introducing trust groups or shared-process pooling.

## Method

1. Build the plugin host in release mode with `cargo build -p volt-plugin-host --release`.
2. Run `node scripts/ci/plugin-startup-bench.mjs`.
3. The harness creates a minimal plugin that:
   - imports `definePlugin` from `volt:plugin`
   - registers a single `ping` command during `activate`
4. Each iteration measures:
   - process spawn to `ready` signal
   - process spawn to `activate` plus the first `plugin:invoke-command` response
5. The benchmark host answers the plugin's `plugin:register-command` request so the measurement uses the real stdio protocol and activation path.

## Current Result

Measured on the Session 6 development machine on March 15, 2026 with 10 iterations:

| Metric | Min | Median | Max | P95 |
| --- | ---: | ---: | ---: | ---: |
| Spawn -> ready | 9.79 ms | 15.48 ms | 16.25 ms | 16.25 ms |
| Spawn -> first command response | 13.97 ms | 17.42 ms | 18.08 ms | 18.08 ms |

Decision threshold:

- spawn -> ready must stay under `200 ms`
- spawn -> first command response must stay under `500 ms`

Current measurements are well below both thresholds, so trust groups remain deferred. The plugin system keeps one-process-per-plugin isolation by default, and shared-process pooling stays an escape hatch until a later benchmark shows a real need.
