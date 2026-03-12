# CLI Reference

The Volt CLI (`volt-cli`) provides commands for developing, building, previewing, and packaging Volt applications.

## `volt dev`

Start the development server with a native window and hot module replacement.

```bash
volt dev [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--port <port>` | Vite dev server port | `5173` |
| `--host <host>` | Vite dev server host | `localhost` |

**Behavior:**
1. Loads `volt.config.ts` from the project root
2. Starts Vite in development mode as a child process
3. Waits for the Vite server to become ready
4. Creates a native window pointing to the dev server URL
5. Enables devtools in the WebView

Runtime/native logging can be tuned with `VOLT_LOG` (or `RUST_LOG` as fallback). Defaults are `debug` for development builds and `warn` for production builds.
6. Falls back to web-only mode if the native binding is unavailable

**Example:**
```bash
volt dev --port 3000
```

## `volt build`

Build the application for production.

```bash
volt build [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--target <target>` | Rust compilation target triple | Host platform |

**Behavior:**
1. Loads `volt.config.ts`
2. Runs `vite build` to produce optimized frontend assets in `dist/`
3. Validates the build output (must contain `index.html`)
4. Creates a binary asset bundle (`.volt-assets.bin`)
5. Compiles the Rust binary with `cargo build --release`, embedding the asset bundle
6. Outputs the final binary to `dist-volt/`

**Example:**
```bash
# Build for the current platform
volt build

# Cross-compile for a specific target
volt build --target x86_64-unknown-linux-gnu
```

## `volt preview`

Preview the production build locally using Vite's preview server.

```bash
volt preview
```

**Behavior:**
1. Reads the `build.outDir` from `volt.config.ts` (defaults to `dist`)
2. Starts a local HTTP server serving the build output
3. Displays the preview URL

This is useful for testing the production build before packaging.

## `volt test`

Run Volt E2E suites defined in `volt.test.config.ts`.

```bash
volt test [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--config <path>` | Path to test config file | auto-detect (`volt.test.config.ts/.mjs/.js`) |
| `--suite <name>` | Run only selected suite(s); repeatable | all suites |
| `--list` | Print configured suites without running | `false` |
| `--timeout <ms>` | Override suite timeout in milliseconds | config value or `120000` |
| `--retries <count>` | Retry failing suites up to N additional attempts | config value or `0` |
| `--artifacts-dir <path>` | Write logs/screenshots/summaries to a fixed artifact directory | `artifacts/volt-test/<timestamp>` |

**Behavior:**
1. Loads the test configuration file.
2. Selects suites (all by default, or filtered with `--suite`).
3. Runs suites sequentially with timeout enforcement.
4. Retries failing suites when configured (`--retries` or `config.retries`).
5. Captures per-attempt artifacts (logs, payload snapshots, screenshots, run/flake summaries).
6. Exits non-zero if a suite still fails after retries.

**Example:**
```bash
# List suites
volt test --list

# Run only ipc-demo smoke
volt test --suite ipc-demo-smoke
```

## `volt doctor`

Validate local signing and packaging prerequisites before running `volt package`.

```bash
volt doctor [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--target <target>` | Target platform (`win32`, `darwin`, `linux`) | Host platform |
| `--format <format>` | Package format (`nsis`, `msix`, `app`, `dmg`, `appimage`, `deb`) | Platform default set |
| `--json` | Print machine-readable doctor report JSON | `false` |

**Behavior:**
1. Loads `volt.config.ts` and resolves packaging/signing settings for the selected platform.
2. Checks packaging tools required by the selected format(s) (for example `makensis`, `makemsix`, `makeappx`, `appimagetool`, `dpkg-deb`).
3. Checks signing prerequisites for configured providers (local, Azure Trusted Signing, DigiCert KeyLocker, macOS signing/notarization).
4. Prints pass/warn/fail results and exits non-zero when any required prerequisite is missing.

**Example:**
```bash
volt doctor --target win32 --format msix
```

## `volt package`

Package the built application into platform-specific installers.

```bash
volt package [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--target <target>` | Target platform | Host platform |
| `--format <format>` | Package format (`nsis`, `msix`, `app`, `dmg`, `appimage`, `deb`) | Platform default |
| `--install-mode <mode>` | Windows install scope (`perMachine`, `perUser`) | `perMachine` |
| `--json` | Print package summary JSON to stdout | `false` |
| `--json-output <path>` | Write package summary JSON file for CI | `-` |

**Behavior:**
1. Requires a completed `volt build` (checks for a runtime artifact in `dist-volt/` and validates it is executable for the selected packaging platform)
2. Reads packaging configuration from `volt.config.ts` (`package` section)
3. Generates platform-specific installer:

| Platform | Output |
|----------|--------|
| Windows | NSIS installer (`.exe`) and/or MSIX package (`.msix`) |
| macOS | `.app` bundle, optionally `.dmg` |
| Linux | AppImage (`.AppImage`) and Debian package (`.deb`) |

If `package.icon` is omitted, Volt generates a placeholder icon for Linux packaging so AppImage and `.desktop` metadata remain valid.

4. If code signing is configured (via `package.signing` in config or `VOLT_*` environment variables), signs the output:
   - **macOS**: Signs the `.app` bundle with `codesign`, optionally notarizes with `notarytool`
   - **Windows local**: Signs with `signtool` or `osslsigncode`
   - **Windows Azure Trusted Signing**: Signs with `signtool` and Azure provider metadata
   - **Windows DigiCert KeyLocker**: Signs with `smctl`
   - Signing is fully opt-in — omit config to skip
5. For Windows targets, writes an `enterprise/` deployment bundle containing ADMX/ADML policy templates and installer scripts (enabled by default; configurable via `package.enterprise`).
6. Emits machine-readable summary JSON when `--json` and/or `--json-output` is used.

See [Code Signing](api/signing.md) for setup instructions.

**Example:**
```bash
volt package
```

## `volt sign setup`

Generate a signing bootstrap template and run quick local prerequisite checks.

```bash
volt sign setup [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--platform <platform>` | Template target (`darwin`, `win32`, `all`) | host platform (`all` on Linux) |
| `--windows-provider <provider>` | Windows provider (`local`, `azureTrustedSigning`, `digicertKeyLocker`) | inferred from config/env or `local` |
| `--output <path>` | Template output path | `.env.signing` |
| `--force` | Overwrite existing output file | `false` |
| `--print` | Print generated template to stdout | `false` |
| `--print-only` | Print template only (no file write) | `false` |

**Example:**
```bash
volt sign setup --platform win32 --windows-provider azureTrustedSigning --output .env.signing.ci
```

## `volt update publish`

Generate and publish update artifacts plus a release manifest.

```bash
volt update publish [options]
```

**Options:**
| Option | Description | Default |
|--------|-------------|---------|
| `--artifacts-dir <dir>` | Directory containing runtime artifacts from `volt build` | `dist-volt` |
| `--out-dir <dir>` | Publish output directory | `dist-update` |
| `--provider <provider>` | Publish provider | `local` |
| `--channel <channel>` | Release channel | `stable` |
| `--base-url <url>` | Base URL used to generate artifact URLs in manifest | — |
| `--manifest-file <name>` | Output manifest file name | `manifest-<channel>.json` |
| `--dry-run` | Run preflight checks without writing files | `false` |

**Behavior:**
1. Validates updater config (`updater.endpoint` and `updater.publicKey`)
2. Requires `VOLT_UPDATE_SIGNATURE` to contain the base64 Ed25519 signature for the final update metadata payload
3. Validates build artifacts in `dist-volt/`
4. Computes artifact SHA-256 and size metadata
5. Generates an update manifest compatible with Volt updater expectations
6. Publishes artifact + manifest through the selected provider (`local` currently)

**Example:**
```bash
volt update publish --channel stable --base-url https://updates.example.com/releases
```

## Configuration

All commands read from `volt.config.ts` (or `volt.config.js` / `volt.config.mjs`). Config files are resolved in this order of precedence:

1. `volt.config.ts` (loaded via jiti or dynamic import)
2. `volt.config.js`
3. `volt.config.mjs`

If no config file is found, default values are used. See [Configuration Reference](configuration.md) for all options.
