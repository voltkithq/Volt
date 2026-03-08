# Code Signing

Sign your Volt application for distribution on macOS and Windows. Signing is opt-in — configure it in `volt.config.ts` and set credentials via environment variables.

## Why Sign?

- **macOS**: Unsigned apps are blocked by Gatekeeper. Users see "app is damaged and can't be opened."
- **Windows**: Unsigned apps trigger SmartScreen warnings. Users see "Windows protected your PC."
- **Linux**: No signing required. Package managers handle trust via repo signatures.

## Configuration

```ts
// volt.config.ts
import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'My App',
  version: '1.0.0',
  package: {
    identifier: 'com.example.myapp',
    signing: {
      macOS: {
        identity: 'Developer ID Application: Your Name (TEAMID)',
        entitlements: './entitlements.plist',
        notarize: true,
        teamId: 'TEAMID',
      },
      windows: {
        provider: 'local',
        certificate: './certs/signing.pfx',
        digestAlgorithm: 'sha256',
        timestampUrl: 'http://timestamp.digicert.com',
      },
    },
  },
});
```

## Environment Variables

Sensitive values (certificates, passwords) should **never** be in config files. Use environment variables instead. Env vars override config file values.

### macOS

| Variable | Description |
|----------|-------------|
| `VOLT_MACOS_SIGNING_IDENTITY` | Signing identity (overrides `signing.macOS.identity`) |
| `VOLT_MACOS_CERTIFICATE` | Base64-encoded .p12 certificate (for CI) |
| `VOLT_MACOS_CERTIFICATE_PASSWORD` | Password for the .p12 certificate |
| `VOLT_APPLE_ID` | Apple ID email (for notarization) |
| `VOLT_APPLE_PASSWORD` | App-specific password (for notarization) |
| `VOLT_APPLE_TEAM_ID` | Apple Developer Team ID (overrides `signing.macOS.teamId`) |

### Windows

| Variable | Description |
|----------|-------------|
| `VOLT_WIN_SIGNING_PROVIDER` | Provider (`local`, `azureTrustedSigning`, `digicertKeyLocker`) |
| `VOLT_WIN_CERTIFICATE` | Path to .pfx certificate file (overrides `signing.windows.certificate`) |
| `VOLT_WIN_CERTIFICATE_PASSWORD` | Password for the .pfx certificate |
| `VOLT_WIN_TIMESTAMP_URL` | Timestamp override for all providers |
| `VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH` | Path to Azure Trusted Signing dlib DLL |
| `VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH` | Path to Azure Trusted Signing metadata JSON |
| `VOLT_AZURE_TRUSTED_SIGNING_ENDPOINT` | Optional Azure endpoint |
| `VOLT_AZURE_TRUSTED_SIGNING_ACCOUNT_NAME` | Optional Azure account name |
| `VOLT_AZURE_TRUSTED_SIGNING_CERT_PROFILE` | Optional Azure certificate profile |
| `VOLT_AZURE_TRUSTED_SIGNING_CORRELATION_ID` | Optional Azure correlation ID |
| `VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS` | DigiCert keypair alias |
| `VOLT_DIGICERT_KEYLOCKER_CERT_FINGERPRINT` | Optional DigiCert certificate fingerprint |
| `VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH` | Optional `smctl` path (defaults to `smctl`) |
| `VOLT_DIGICERT_KEYLOCKER_TIMESTAMP_URL` | Optional DigiCert timestamp URL |

## Bootstrap Template

Use `volt sign setup` to generate a provider-aware template:

```bash
volt sign setup --platform win32 --windows-provider azureTrustedSigning --output .env.signing.ci
```

The command generates env var placeholders and prints a quick prerequisite check for required tools.

## macOS Signing

### Prerequisites

1. An **Apple Developer account** ($99/year)
2. A **Developer ID Application** certificate in your Keychain
3. **Xcode Command Line Tools** installed (`xcode-select --install`)

### What Happens

When `volt package` runs with macOS signing configured:

1. **Certificate import** (CI only): If `VOLT_MACOS_CERTIFICATE` is set, the base64-encoded .p12 is imported into a temporary keychain
2. **Code signing**: `codesign` signs the .app bundle with hardened runtime and timestamp
3. **Verification**: `codesign --verify` confirms the signature is valid
4. **Notarization** (if enabled): The app is zipped, submitted to Apple via `notarytool`, and the notarization ticket is stapled
5. **Cleanup**: Temporary keychain is removed (CI mode)

### Entitlements

If your app needs specific entitlements (e.g., network access, JIT), create an entitlements file:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.cs.allow-jit</key>
  <true/>
  <key>com.apple.security.cs.allow-dyld-environment-variables</key>
  <true/>
</dict>
</plist>
```

### Getting an App-Specific Password

Notarization requires an app-specific password (not your Apple ID password):

1. Go to [appleid.apple.com](https://appleid.apple.com)
2. Sign In > App-Specific Passwords > Generate
3. Store the password as `VOLT_APPLE_PASSWORD`

## Windows Signing

### Prerequisites

- **Local provider**: code signing certificate (.pfx/.p12) + `signtool.exe` (Windows SDK) or `osslsigncode`
- **Azure Trusted Signing provider**: `signtool.exe` + Azure Trusted Signing dlib/metadata files
- **DigiCert KeyLocker provider**: `smctl` configured with a keypair alias

### What Happens

When `volt package` runs with Windows signing configured:

1. The **main binary** is signed before being packaged into the installer.
2. The **installer .exe** is signed after NSIS creates it (when NSIS output exists).
3. Provider-specific signing metadata is used based on `windows.provider`.
4. Results are included in machine-readable summary output (`--json` / `--json-output`).

### Signing Tools

| Tool | Provider | Notes |
|------|----------|-------|
| `signtool.exe` | `local`, `azureTrustedSigning` | Preferred on Windows. Part of the Windows SDK. |
| `osslsigncode` | `local` | Cross-platform alternative for local certificate signing. |
| `smctl` | `digicertKeyLocker` | DigiCert KeyLocker CLI used for remote signing. |

Volt automatically detects which tool is available. On Windows, `signtool` is preferred. On macOS/Linux, `osslsigncode` is used.

### Digest Algorithm

Default is `sha256`. Supported values: `sha256`, `sha384`, `sha512`. SHA-1 is not supported (deprecated).

### Timestamp Server

Default is `http://timestamp.digicert.com`. The timestamp ensures the signature remains valid after the certificate expires. Common alternatives:

- `http://timestamp.sectigo.com`
- `http://timestamp.globalsign.com`

## CI/CD Example (GitHub Actions)

```yaml
name: Build & Sign

on:
  push:
    tags: ['v*']

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: volt build
      - name: Package & Sign
        env:
          VOLT_MACOS_SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
          VOLT_MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
          VOLT_MACOS_CERTIFICATE_PASSWORD: ${{ secrets.MACOS_CERTIFICATE_PASSWORD }}
          VOLT_APPLE_ID: ${{ secrets.APPLE_ID }}
          VOLT_APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          VOLT_APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        run: volt package --format dmg

  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: volt build
      - name: Package & Sign
        env:
          VOLT_WIN_CERTIFICATE: ${{ secrets.WIN_CERTIFICATE }}
          VOLT_WIN_CERTIFICATE_PASSWORD: ${{ secrets.WIN_CERTIFICATE_PASSWORD }}
        run: volt package --format nsis
```

## Troubleshooting

### macOS: "No identity found"

Your signing identity isn't in the Keychain. Run `security find-identity -v -p codesigning` to list available identities.

### macOS: Notarization fails

- Ensure `VOLT_APPLE_ID`, `VOLT_APPLE_PASSWORD`, and `VOLT_APPLE_TEAM_ID` are all set
- The password must be an **app-specific password**, not your Apple ID password
- Check the notarization log: `xcrun notarytool log <submission-id> --apple-id <id> --password <pw> --team-id <team>`

### Windows: "signtool not found"

Install the Windows SDK, or add `signtool.exe` to your PATH. Typical location: `C:\Program Files (x86)\Windows Kits\10\bin\<version>\x64\signtool.exe`

### Windows Azure Trusted Signing: missing dlib/metadata

Set both `VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH` and `VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH` (or the matching config fields). Both files are required.

### Windows DigiCert KeyLocker: missing keypair alias

Set `VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS` (or `package.signing.windows.digicertKeyLocker.keypairAlias`).

### Windows: Cross-platform signing

Install `osslsigncode` on macOS/Linux to sign Windows executables without a Windows machine:
- macOS: `brew install osslsigncode`
- Ubuntu: `apt install osslsigncode`

### No signing tools available

If neither `signtool` nor `osslsigncode` is found, `volt package` will throw an error. Install one of them to proceed.

### Signing is skipped silently

Signing is opt-in. If you don't set any signing config or env vars, `volt package` works exactly as before — no signing, no warnings.
