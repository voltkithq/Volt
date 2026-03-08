# App

Application lifecycle management.

## `createApp(config)`

Create and return the global `VoltApp` instance. Can only be called once.

```ts
import { createApp } from 'voltkit';

const app = createApp({
  name: 'My App',
  version: '1.0.0',
});
```

**Parameters:**
- `config: VoltConfig` — Application configuration

**Returns:** `VoltApp`

**Throws:** If called more than once.

## `getApp()`

Get the existing global `VoltApp` instance.

```ts
import { getApp } from 'voltkit';

const app = getApp();
console.log(app.getName());
```

**Returns:** `VoltApp`

**Throws:** If `createApp()` has not been called.

## `VoltApp`

Extends `EventEmitter`. Manages the app lifecycle.

### Methods

#### `getName(): string`
Returns the application name from config.

#### `getVersion(): string`
Returns the version string. Defaults to `'0.0.0'` if not configured.

#### `getConfig(): VoltConfig`
Returns the full application configuration object.

#### `ready: boolean` (getter)
Whether the app has finished initializing.

#### `markReady(): void`
Mark the app as ready and emit the `'ready'` event. Calling multiple times has no effect after the first.

#### `whenReady(): Promise<void>`
Returns a promise that resolves when the app is ready. Resolves immediately if already ready.

```ts
await app.whenReady();
// App is now initialized
```

#### `quit(): void`
Emit `'before-quit'` then `'quit'` events.

### Events

| Event | Description |
|-------|-------------|
| `'ready'` | App initialization is complete |
| `'before-quit'` | App is about to quit |
| `'quit'` | App is quitting |
| `'window-all-closed'` | All windows have been closed |

## Native Bridge Event Payloads

When using `VoltApp.onEvent(...)` from the native bridge, payloads are JSON strings with the following shapes:

- `{"type":"quit"}`
- `{"type":"window-closed","windowId":"<native-window-id>","jsWindowId":"<browser-window-id-or-null>"}`
- `{"type":"ipc-message","windowId":"<browser-window-id>","raw":<ipc-request-object-or-string>}`
- `{"type":"menu-event","menuId":"<menu-item-id>"}`
- `{"type":"shortcut-triggered","id":<number>}`
