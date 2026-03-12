# Configuration Reference

Volt applications are configured via `volt.config.ts` at the project root. Use `defineConfig()` for type checking.

```ts
import { defineConfig } from 'voltkit';

export default defineConfig({
  // ... options
});
```

## Full Schema

### `name` (required)

**Type:** `string`

The display name of the application. Used in window titles, installers, and system integrations.

```ts
name: 'My App'
```

### `version`

**Type:** `string` | Default: `'0.0.0'`

Application version in semver format. Used by the auto-updater for version comparison.

```ts
version: '1.2.3'
```

### `backend`

**Type:** `string` | Default: first existing of `src/backend.ts`, `src/backend.js`, `backend.ts`, `backend.js`

Optional backend entrypoint bundled into the runtime Boa context.

```ts
backend: './src/backend.ts'
```

When omitted, Volt auto-detects backend entry candidates in this order:

1. `src/backend.ts`
2. `src/backend.js`
3. `backend.ts`
4. `backend.js`

If none exist, the app runs without a backend bundle.

### `window`

**Type:** `WindowOptions`

Default window configuration applied when creating `BrowserWindow` instances.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `width` | `number` | `800` | Window width in pixels |
| `height` | `number` | `600` | Window height in pixels |
| `minWidth` | `number` | — | Minimum window width |
| `minHeight` | `number` | — | Minimum window height |
| `maxWidth` | `number` | — | Maximum window width |
| `maxHeight` | `number` | — | Maximum window height |
| `title` | `string` | `'Volt'` | Window title |
| `resizable` | `boolean` | `true` | Whether the window can be resized |
| `decorations` | `boolean` | `true` | Whether to show window decorations (title bar, borders) |
| `transparent` | `boolean` | `false` | Whether the window background is transparent |
| `alwaysOnTop` | `boolean` | `false` | Whether the window stays above all others |
| `maximized` | `boolean` | `false` | Whether the window starts maximized |
| `x` | `number` | — | Initial X position (centered if omitted) |
| `y` | `number` | — | Initial Y position (centered if omitted) |

```ts
window: {
  width: 1024,
  height: 768,
  title: 'My App',
  minWidth: 400,
  minHeight: 300,
  resizable: true,
}
```

### `permissions`

**Type:** `Permission[]` | Default: `[]`

Capabilities the app needs. APIs not listed here will throw an error at runtime.

| Permission | Grants Access To |
|------------|------------------|
| `'clipboard'` | `clipboard.readText()`, `writeText()`, `readImage()`, `writeImage()` |
| `'notification'` | `new Notification().show()` |
| `'dialog'` | `dialog.showOpenDialog()`, `showSaveDialog()`, `showMessageBox()` |
| `'fs'` | `fs.readFile()`, `writeFile()`, `readDir()`, `stat()`, `mkdir()`, `remove()` |
| `'db'` | `volt:db` backend module (`open()`, `execute()`, `query()`, `transaction()`, `close()`) |
| `'menu'` | Menu APIs (`Menu`, `MenuItem`, and `setAppMenu`) |
| `'shell'` | `shell.openExternal()` |
| `'http'` | `volt:http` backend module (`fetch()`) |
| `'globalShortcut'` | `globalShortcut.register()`, `unregister()`, etc. |
| `'tray'` | `new Tray()` |
| `'secureStorage'` | `volt:secureStorage` backend module (`set()`, `get()`, `delete()`, `has()`) |

```ts
permissions: ['clipboard', 'dialog', 'fs', 'db', 'http']
```

### `build`

**Type:** `BuildConfig`

Build output configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `outDir` | `string` | `'dist'` | Output directory for Vite build |

```ts
build: {
  outDir: 'build'
}
```

### `package`

**Type:** `PackageConfig`

Packaging configuration for platform-specific installers.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `identifier` | `string` | Yes | Unique app identifier (e.g., `'com.example.myapp'`) |
| `icon` | `string` | No | Path to the application icon. If omitted, Linux packages use a generated placeholder icon. |
| `categories` | `string[]` | No | App categories (for Linux `.desktop` files) |
| `windows` | `PackageWindowsConfig` | No | Windows packaging options (`installMode`, `silentAllUsers`, MSIX metadata) |
| `enterprise` | `PackageEnterpriseConfig` | No | Enterprise output controls for ADMX/docs bundle generation |

```ts
package: {
  identifier: 'com.example.myapp',
  icon: './assets/icon.png',
  categories: ['Utility'],
  windows: {
    installMode: 'perMachine',
    silentAllUsers: true,
    msix: {
      identityName: 'com.example.myapp',
      publisher: 'CN=Example Corp',
    },
  },
  enterprise: {
    generateAdmx: true,
    includeDocsBundle: true,
  },
}
```

#### `package.windows`

**Type:** `PackageWindowsConfig` | Optional

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `installMode` | `'perMachine' \| 'perUser'` | `'perMachine'` | Default NSIS install scope and enterprise policy default |
| `silentAllUsers` | `boolean` | `true` for per-machine mode | Forces `/ALLUSERS=1` for silent NSIS installs |
| `msix` | `PackageWindowsMsixConfig` | — | Optional MSIX identity/display overrides |

#### `package.windows.msix`

| Field | Type | Description |
|-------|------|-------------|
| `identityName` | `string` | Stable package identity used by Windows appx/MSIX updates |
| `publisher` | `string` | Publisher subject (usually CN format) |
| `publisherDisplayName` | `string` | Publisher label shown in Windows package metadata |
| `displayName` | `string` | App display name shown in package metadata |
| `description` | `string` | Package description |

#### `package.enterprise`

**Type:** `PackageEnterpriseConfig` | Optional

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `generateAdmx` | `boolean` | `true` | Generate ADMX/ADML policy templates and `policy-values.json` |
| `includeDocsBundle` | `boolean` | `true` | Generate `enterprise/DEPLOYMENT.md` and install scripts |

When `package.enterprise` is omitted, Windows packaging keeps both enterprise outputs enabled by default.

#### `package.signing`

**Type:** `SigningConfig` | Optional

Code signing configuration for macOS and Windows. Fully opt-in — omit to skip signing.

| Field | Type | Description |
|-------|------|-------------|
| `macOS` | `MacOSSigningConfig` | macOS code signing and notarization |
| `windows` | `WindowsSigningConfig` | Windows Authenticode signing |

**macOS options:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `identity` | `string` | — | Signing identity. Overridden by `VOLT_MACOS_SIGNING_IDENTITY` env var. |
| `entitlements` | `string` | — | Path to entitlements .plist file |
| `notarize` | `boolean` | `true` | Whether to notarize after signing |
| `teamId` | `string` | — | Apple Team ID. Overridden by `VOLT_APPLE_TEAM_ID` env var. |

**Windows options:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `'local' \| 'azureTrustedSigning' \| 'digicertKeyLocker'` | `'local'` | Windows signing backend provider |
| `certificate` | `string` | — | Path to .pfx file. Overridden by `VOLT_WIN_CERTIFICATE` env var. |
| `digestAlgorithm` | `string` | `'sha256'` | Hash algorithm (`sha256`, `sha384`, `sha512`) |
| `timestampUrl` | `string` | `'http://timestamp.digicert.com'` | RFC 3161 timestamp server |
| `azureTrustedSigning` | `AzureTrustedSigningConfig` | — | Azure provider settings (`dlibPath`, `metadataPath`, optional endpoint metadata) |
| `digicertKeyLocker` | `DigiCertKeyLockerConfig` | — | DigiCert provider settings (`keypairAlias`, optional fingerprint/tooling overrides) |

Sensitive values (passwords, certificates) should be set via environment variables. See [Code Signing](api/signing.md) for the full guide.

```ts
package: {
  identifier: 'com.example.myapp',
  signing: {
    macOS: {
      identity: 'Developer ID Application: Your Name (TEAMID)',
      entitlements: './entitlements.plist',
    },
    windows: {
      provider: 'local',
      certificate: './certs/signing.pfx',
    },
  },
}
```

### `updater`

**Type:** `UpdaterConfig`

Auto-updater configuration. Both fields are required if the section is present.
The update service response/status contract is documented in [Auto Updater API](api/updater.md#update-endpoint-contract).

| Field | Type | Description |
|-------|------|-------------|
| `endpoint` | `string` | URL to check for updates (must be HTTPS in production) |
| `publicKey` | `string` | Ed25519 public key for signature verification (base64) |
| `telemetry` | `UpdaterTelemetryConfig` | Optional updater telemetry controls (disabled by default) |

#### `updater.telemetry`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `boolean` | `false` | Enables update lifecycle telemetry events and sink emission |
| `sink` | `'none' \| 'stdout'` | `'none'` | Telemetry sink adapter (`none` is no-op) |

```ts
updater: {
  endpoint: 'https://updates.example.com/check',
  publicKey: 'base64-encoded-ed25519-public-key',
  telemetry: {
    enabled: true,
    sink: 'stdout',
  },
}
```

### `devtools`

**Type:** `boolean` | Default: `true` in dev, `false` in production

Whether to enable WebView devtools.

```ts
devtools: false
```

## Runtime Logging

Rust runtime logs use structured severity levels and support filter configuration through environment variables:

- `VOLT_LOG` (preferred)
- `RUST_LOG` (fallback)

Default level behavior:

- Development builds: `debug`
- Production/release builds: `warn`

Examples:

```bash
VOLT_LOG=info
VOLT_LOG=warn,volt_runner=debug
```

## Complete Example

```ts
import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'Acme Desktop',
  version: '2.1.0',
  window: {
    width: 1280,
    height: 800,
    title: 'Acme',
    minWidth: 800,
    minHeight: 600,
  },
  permissions: ['clipboard', 'notification', 'tray', 'globalShortcut', 'shell', 'secureStorage'],
  build: {
    outDir: 'dist',
  },
  package: {
    identifier: 'com.acme.desktop',
    icon: './assets/icon.png',
    categories: ['Network', 'Chat'],
  },
  updater: {
    endpoint: 'https://updates.acme.dev/check',
    publicKey: 'MCowBQYDK2VwAyEA...',
  },
  devtools: false,
});
```

## Validation

The CLI validates configuration at load time and reports errors:

- Missing or empty `name` → defaults to `'Volt App'` with a warning
- Non-string `version` → version is cleared with a warning
- Non-positive `width`/`height` → reset to defaults (800/600)
- Unknown permission names → ignored by runtime permission parsing and do not prevent startup
- Missing `updater.endpoint` or `updater.publicKey` → logged as errors
- Invalid `updater.telemetry` values → telemetry falls back to disabled/no-op behavior
- Signing: missing identity/certificate without env var → logged as warning
- Signing: invalid `digestAlgorithm` → logged as error
- Signing: entitlements path not found → logged as error
