# Runtime Model

This document defines Volt's current threading and runtime model (v0.1) and the constraints it introduces for API behavior.

## Core Constraint

- `tao::EventLoop` remains thread-affine.
- On Windows/Linux/BSD, Volt runs the native loop on a dedicated native thread.
- On macOS, `tao/wry` main-thread constraints keep the native loop on the caller thread.
- Node.js callback execution and native event-loop execution are coordinated by channels, not a shared async runtime.

## Runtime Mode Contract

- Runtime mode values:
  - `main-thread-macos`
  - `split-runtime-threaded`
- Platform mapping:
  - macOS -> `main-thread-macos`
  - non-macOS (`linux`, `windows`, `freebsd`, etc.) -> `split-runtime-threaded`
- Contract assertions:
  - Rust: `crates/volt-napi/src/app.rs` tests `test_runtime_mode_for_target_mapping` and `test_runtime_mode_for_current_target_matches_cfg`
  - CLI: `packages/volt-cli/src/__tests__/runtime-mode.test.ts`
  - CI matrix: `.github/workflows/ci.yml` `platform-behavior` job runs runtime-mode checks on all OS runners

## Threads and Responsibilities

- Native loop thread (Windows/Linux/BSD):
  - Runs `tao` event loop.
  - Owns window/webview/menu/shortcut OS resources.
  - Drains `AppCommand` queue and performs native operations.
- Bridge thread:
  - Receives typed dispatch events from the native loop.
  - Dispatches `ThreadsafeFunction` callbacks to Node.js (`onEvent` and shortcut callbacks).
- Node.js thread:
  - Hosts framework APIs (`voltkit`, `volt-cli` runtime wiring).
  - Receives native app events through N-API callbacks.
  - Produces `AppCommand` requests via the command bridge.

## Cross-Thread Message Flows

- Command path (Node.js -> native loop):
  - `AppCommand` via channel envelope with `trace_id` and enqueue timestamp.
  - Event loop wakeup via `AppEvent::ProcessCommands`.
- Notification path (native loop -> Node.js):
  - Native loop emits typed bridge dispatch events.
  - Bridge thread forwards these through N-API `ThreadsafeFunction`.

## Lifecycle and Shutdown Semantics

- Bridge lifecycle is explicit and guarded:
  - Initialized once per running app loop.
  - Shutdown deactivates bridge and detaches global context.
- On shutdown:
  - registered global shortcuts are unregistered
  - app menu handle is released
  - bridge is shut down so new commands fail fast
  - callback registries are cleared

## Window Lifecycle Model

Each window transitions through:

- `Active`
- `Closing`
- `Closed`

Invariants are checked in debug assertions to keep:

- window store
- `js_to_tao`
- `tao_to_js`
- window lifecycle state map

structurally consistent after create/close/quit transitions.

## IPC Behavior Notes

- WebView -> Node.js IPC ingress is event-driven through the bridge.
- Response path to renderer uses evaluated response scripts.
- Timeout/error classification is currently enforced in the Node-side IPC pipeline;
  native handler execution is synchronous.
- IPC abuse bounds are enforced in bridge/runtime layers (payload size and in-flight caps).
- Reserved `volt:native:*` channels can be resolved in the bridge/runtime layers before Boa
  handler dispatch; payload bounds, prototype-pollution checks, and rate limiting still apply.
- `volt-cli dev` supports an out-of-process native host bridge mode as an explicit opt-in (`VOLT_NATIVE_HOST=1`).

## Observability Hooks

- Per-command `trace_id`
- Queue delay and processing duration logging
- Counters:
  - sent commands
  - processed commands
  - failed command sends
- Dropped callback dispatch counter in N-API layer



## Known Architectural Limits in v0.1

- macOS still runs the native loop on the caller thread due platform requirements.
- Native and Node runtimes are coordinated by channels and callback boundaries, not a unified scheduler.
- IPC handler timeout enforcement remains Node-side; native handler registration is synchronous.
- Out-of-process host mode is currently implemented for CLI development flow first and remains opt-in while host-protocol parity is expanded.
