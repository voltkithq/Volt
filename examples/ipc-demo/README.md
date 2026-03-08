# IPC Demo

`ipc-demo` is the reference Volt app that exercises renderer-to-backend IPC plus native runtime modules.

It now includes a typed IPC contract migration sample:

- Contract file: `src/ipc-contract.ts`
- Backend registration: `registerContractHandlers(...)` with runtime request/response validation
- Renderer usage: `createContractInvoker(...)` plus `createLegacyInvokeAdapter(...)` for old channel names

## Quick Start

From repo root:

```bash
pnpm --filter ipc-demo dev
```

Build standalone artifact:

```bash
pnpm --filter ipc-demo build
```

## Secure Storage Flow

This demo includes a `Secure Storage (volt:secureStorage)` panel wired through backend IPC handlers:

- `secure-storage:set`
- `secure-storage:get`
- `secure-storage:has`
- `secure-storage:delete`

Default demo key: `ipc-demo/demo-secret`.

Manual verification helper:

```bash
pnpm --filter ipc-demo verify:secure-storage:manual
```

The helper validates source/config wiring and prints a manual UI checklist.
