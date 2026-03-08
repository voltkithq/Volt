# IPC (Inter-Process Communication)

Communication between the main process (Node.js) and the renderer (WebView).

## Main Process API

### `ipcMain.handle(channel, handler)`

Register a handler for an IPC channel.

```ts
import { ipcMain } from 'voltkit';

ipcMain.handle('get-user', async (args) => {
  const { id } = args as { id: number };
  return { name: 'Alice', id };
});
```

**Parameters:**
- `channel: string` — Channel name
- `handler: (args: unknown) => Promise<unknown> | unknown` — Handler function

**Throws:** If a handler is already registered for the channel.

### `ipcMain.removeHandler(channel)`

Remove a previously registered handler.

```ts
ipcMain.removeHandler('get-user');
```

### `ipcMain.hasHandler(channel): boolean`

Check if a handler is registered for a channel.

### `ipcMain.processRequest(id, method, args)`

Process an IPC request. Used internally by the native bridge.

**Returns:** `Promise<{ id: string; result?: unknown; error?: string; errorCode?: IpcErrorCode; errorDetails?: unknown }>`

### `IpcErrorCode`

Stable error codes for IPC failures:

- `IPC_HANDLER_NOT_FOUND`
- `IPC_HANDLER_ERROR`
- `IPC_HANDLER_TIMEOUT`
- `IPC_PAYLOAD_TOO_LARGE`
- `IPC_IN_FLIGHT_LIMIT`

### Timeout behavior

`ipcMain.processRequest` uses a bounded handler timeout (default `5000ms`) to avoid hanging the native reply path.
An optional fourth argument `{ timeoutMs }` can override this when used internally.

## Renderer API

These functions are available in the WebView context via `window.__volt__`.

### `invoke<T>(channel, ...args): Promise<T>`

Invoke a main-process handler from the renderer.

```ts
import { invoke } from 'voltkit/renderer';

const user = await invoke<{ name: string }>('get-user', { id: 1 });
console.log(user.name); // 'Alice'
```

**Throws:** If called outside the renderer context (no `window.__volt__`).

### `on(event, callback): void`

Listen for events emitted from the main process.

```ts
import { on } from 'voltkit/renderer';

on('notification', (data) => {
  console.log('Received:', data);
});
```

### `off(event, callback): void`

Remove an event listener.

## Typed IPC Contracts

Volt also provides an opt-in typed contract layer for channel definitions, compile-time inference, and runtime request/response validation.

### Contract Definition

```ts
import { IpcSchema, defineCommands } from 'voltkit/ipc-contract';

export const commands = defineCommands({
  'demo.compute': {
    request: IpcSchema.object({
      a: IpcSchema.number(),
      b: IpcSchema.number(),
    }, 'ComputeArgs'),
    response: IpcSchema.object({
      sum: IpcSchema.number(),
    }, 'ComputeResult'),
    aliases: ['compute'],
  },
});
```

### Backend Registration

```ts
import { ipcMain } from 'volt:ipc';
import { registerContractHandlers } from 'voltkit/ipc-contract';
import { commands } from './ipc-contract';

registerContractHandlers(ipcMain, commands, {
  'demo.compute': ({ a, b }) => ({ sum: a + b }),
});
```

### Renderer Invocation

```ts
import { createContractInvoker } from 'voltkit/renderer';
import { commands } from './ipc-contract';

const typedIpc = createContractInvoker(commands, (channel, args) =>
  window.__volt__!.invoke(channel, args),
);

const result = await typedIpc.invoke('demo.compute', { a: 2, b: 3 });
// result is inferred as { sum: number }
```

### Legacy Compatibility Adapter

If you need to keep old channel names while migrating, use aliases plus the legacy adapter:

```ts
import { createLegacyInvokeAdapter } from 'voltkit/renderer';
import { commands } from './ipc-contract';

const invokeLegacy = createLegacyInvokeAdapter(
  commands,
  (channel, args) => window.__volt__!.invoke(channel, args),
);

await invokeLegacy('compute', { a: 2, b: 3 }); // resolves to demo.compute
```

`registerContractHandlers` registers both canonical and legacy alias channels, so migration can be incremental.

`defineCommands(...)` validates alias collisions up-front and throws immediately for duplicate alias mappings.

### Guard Caveat

`createSchema<T>(...)` uses user-defined guards, and TypeScript cannot prove the guard and `T` are perfectly aligned.
Prefer `IpcSchema` helpers (`null`, `string`, `number`, `boolean`, `object`, `array`, `literal`, `optional`) when possible.

### Error Handling Pattern

```ts
import { isIpcContractValidationError } from 'voltkit/renderer';

try {
  await typedIpc.invoke('demo.compute', { a: 2, b: 3 });
} catch (error) {
  if (isIpcContractValidationError(error)) {
    console.error(error.channel, error.phase, error.message);
  } else {
    console.error(error);
  }
}
```

## Security

- All IPC messages are checked for prototype pollution (`__proto__`, `constructor`, `prototype`)
- A rate limiter (1000 req/s) prevents IPC flooding
- Oversized request payloads are rejected with `IPC_PAYLOAD_TOO_LARGE`
- Per-window in-flight IPC load is capped; overflow is rejected with `IPC_IN_FLIGHT_LIMIT`
- Response payloads are escaped before injection into the WebView
- Timeout failures return `errorCode: "IPC_HANDLER_TIMEOUT"` with `errorDetails`
