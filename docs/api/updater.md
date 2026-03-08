# Auto Updater

Automatic updates with Ed25519 signature verification. Requires `updater` configuration in `volt.config.ts`.

## Configuration

```ts
// volt.config.ts
export default defineConfig({
  name: 'My App',
  version: '1.0.0',
  updater: {
    endpoint: 'https://updates.example.com/check',
    publicKey: 'base64-encoded-ed25519-public-key',
  },
});
```

## `autoUpdater`

Extends `EventEmitter`. Singleton instance.

### `autoUpdater.checkForUpdates(): Promise<UpdateInfo | null>`

Check the configured endpoint for available updates.

```ts
import { autoUpdater } from 'voltkit';

autoUpdater.on('update-available', (info) => {
  console.log(`Update available: ${info.version}`);
});

const info = await autoUpdater.checkForUpdates();
```

**Returns:** `UpdateInfo` if an update is available, `null` when no update is available.

**Throws:** Rejects on updater/network/parse errors (and also emits the `'error'` event).

### Update Endpoint Contract

`checkForUpdates()` requests `updater.endpoint` with query params:

- `current_version`: current app version (URL-encoded)
- `target`: runtime target (for example `windows-x64`, `darwin-x64`, `darwin-arm64`, `linux-x64`, `unknown`)

The endpoint must follow this response contract:

- `200 OK`: update available; response body must be valid JSON matching `UpdateInfo`
- `204 No Content`: no update available
- `404 Not Found`: treated as no update available (useful for simple/static update routing)
- any other status: treated as updater error

Example `200` response:

```json
{
  "version": "1.2.0",
  "url": "https://updates.example.com/downloads/my-app-1.2.0.exe",
  "signature": "base64-ed25519-signature-over-version-url-sha256",
  "sha256": "hex-encoded-sha256"
}
```

### `autoUpdater.downloadUpdate(info): Promise<void>`

Download an update and verify its Ed25519 signature and SHA-256 hash.

```ts
autoUpdater.on('update-downloaded', (info) => {
  console.log(`Downloaded: ${info.version}`);
});

await autoUpdater.downloadUpdate(info);
```

**Parameters:**
- `info: UpdateInfo` — Update information from `checkForUpdates()`

**Throws:** If download or verification fails.

### `autoUpdater.quitAndInstall(): void`

Quit the application and apply the downloaded update.

On Windows, Volt uses an external updater helper binary (`volt-updater-helper.exe`) to replace
the executable after the app process exits.

```ts
autoUpdater.quitAndInstall();
```

**Throws:** If no update has been downloaded.

## Events

| Event | Payload | Description |
|-------|---------|-------------|
| `'checking-for-update'` | — | Started checking for updates |
| `'update-available'` | `UpdateInfo` | A new version is available |
| `'update-not-available'` | — | Already on the latest version |
| `'update-downloaded'` | `UpdateInfo` | Download and verification complete |
| `'error'` | `Error` | An error occurred |

### Telemetry Event Stream

When `updater.telemetry.enabled` is `true`, the runtime emits `update:telemetry`
events with schema:

```json
{
  "schemaVersion": 1,
  "event": "update:lifecycle",
  "version": "1.2.3",
  "stage": "download|marker|install",
  "status": "start|success|failure|cancelled",
  "timestampUnixMs": 1730000000000,
  "detail": ""
}
```

Data policy: updater telemetry contains only lifecycle metadata (no user identifiers).

## Types

### `UpdateInfo`

```ts
interface UpdateInfo {
  version: string;    // New version (semver)
  url: string;        // Download URL
  signature: string;  // Ed25519 signature over version/url/sha256 (base64)
  sha256: string;     // SHA-256 hash (hex)
}
```

## Security

The update pipeline verifies:

1. **HTTPS** - Both the update-check endpoint and download URL must use HTTPS (HTTP allowed only for localhost in development/testing)
2. **Ed25519 metadata signature** — `version`, `url`, and `sha256` are signed together; public key is in `volt.config.ts`
3. **SHA-256 hash** — Downloaded bytes are hashed and compared against the signed `sha256`
4. **Semver check** — New version must be greater than current (prevents downgrade attacks)
