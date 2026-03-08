# Security Model

Volt implements defense-in-depth security across multiple layers. This document describes the threat model and mitigations.

## Permission System

Volt uses a **capability-based permission model**. APIs that access system resources require explicit declaration in `volt.config.ts`:

```ts
export default defineConfig({
  name: 'My App',
  permissions: ['clipboard', 'fs', 'db', 'http', 'shell'],
});
```

**Valid permissions:** `clipboard`, `notification`, `dialog`, `fs`, `db`, `menu`, `shell`, `http`, `globalShortcut`, `tray`, `secureStorage`

Permissions are loaded at startup and are immutable. The `CapabilityGuard` enforces checks at the Rust layer before any native operation executes. Undeclared capabilities produce a clear error message.

**Default-deny:** With no permissions declared, all native APIs are blocked.

### Secure Storage Threat Model

`volt:secureStorage` is backend-only and requires `permissions: ['secureStorage']`.

- Trust boundary: renderer code must go through backend IPC handlers before any credential operation.
- Storage medium: credentials are delegated to OS keychain/keyring providers on supported platforms.
- Key scope: entries are namespaced by app identity to avoid cross-app key collisions.
- CI fallback: tests may use in-memory backend overrides, but production builds should use native keyrings.
- Non-goal: this layer protects at-rest credential storage, not runtime compromise of backend JavaScript code.

## Content Security Policy (CSP)

### Production
```
default-src 'none';
script-src 'self' volt://localhost https://volt.localhost;
style-src 'self' 'unsafe-inline' volt://localhost https://volt.localhost;
img-src 'self' data: volt://localhost https://volt.localhost;
font-src 'self' volt://localhost https://volt.localhost;
connect-src 'self' volt://localhost https://volt.localhost
```

- No `unsafe-eval` - prevents XSS via `eval()` or `Function()`
- No wildcard sources - all resources must come from the app bundle
- `data:` allowed only for images (inline icons, favicons)

### Development
The dev CSP adds the Vite dev server origin to allow HMR:
```
script-src 'self' volt://localhost https://volt.localhost http://localhost:5173;
connect-src 'self' volt://localhost https://volt.localhost http://localhost:5173 ws://localhost:5173;
```
If your dev server uses HTTPS, Volt uses `wss://...` for the websocket origin.

## Filesystem Sandboxing

All filesystem operations go through `safe_resolve()`, which enforces:

1. **No absolute paths** - `/etc/passwd`, `C:\Windows\System32`, `\server\share` are all rejected
2. **No path traversal** - `../../secret` is blocked by checking each path component for `..`
3. **Canonicalization check** - After resolving, the canonical path must start with the canonical base directory
4. **Reserved device names** - Windows device names (`CON`, `PRN`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`) are blocked to prevent device access attacks
5. **Scoped create path checks** - Parent directories for create/write flows are materialized inside the sandbox one component at a time, and symlink escapes are rejected before a new file is created

The TypeScript `fs` module adds a second validation layer before calling into Rust, providing defense-in-depth.

## IPC Security

### Prototype Pollution Protection

All incoming IPC messages are scanned for dangerous keys before processing:

- **Raw string scan** - Fast check for `"__proto__"`, `"constructor"`, `"prototype"` in the raw JSON
- **Recursive value check** - After parsing, all nested objects and arrays are walked to detect pollution keys

This prevents attacks where a malicious frontend could inject `__proto__` into handler arguments.

### Rate Limiting

A sliding-window rate limiter (default: 1000 requests/second) protects against IPC flooding. The limiter is shared across the runtime pool, so the cap applies to aggregate IPC traffic, not per-worker traffic. Expired entries are cleaned up on each check.

### Payload and Queue Bounds

IPC uses bounded load-shedding to prevent memory growth under abusive traffic:

- Payload size is capped (`256 KiB`); oversized messages are rejected with `IPC_PAYLOAD_TOO_LARGE`
- Per-window in-flight IPC processing is capped (`32` by default in dev bridge); overflow is rejected with `IPC_IN_FLIGHT_LIMIT`
- Renderer pending IPC map is bounded (`128` pending requests) to avoid unbounded queue buildup
- Bridge workers enforce end-to-end handler timeouts so a wedged synchronous handler cannot stall the IPC queue indefinitely

### Response Escaping

IPC responses are embedded in JavaScript via `evaluate_script()`. All response JSON is escaped (backslashes, single quotes, newlines) to prevent injection through crafted payloads.

## Navigation Whitelist

The `WebViewConfig` controls which URLs the WebView can navigate to:

- The configured `source` origin - Automatically allowed so the app can load its declared page
- `about:blank` and `data:` URLs - Allowed for internal use
- `volt://` protocol - Allowed for serving embedded assets
- Declared origins - Explicitly listed in `allowed_origins`
- Everything else - **Blocked by default**

## Shell URL Validation

`shell.openExternal()` only allows safe URL schemes:

- **Allowed:** `http`, `https`, `mailto`
- **Blocked:** `file`, `javascript`, `data`, `vbscript`, `ftp`, `smb`, `ssh`, and all others

Validation happens at both the TypeScript layer (using `URL` parser) and the Rust layer (using the `url` crate).

## Auto-Update Security

The update pipeline verifies integrity at multiple levels:

1. **HTTPS transport** - Update endpoint must use HTTPS (HTTP allowed only for localhost during development)
2. **Ed25519 metadata signature** - `version`, `url`, and `sha256` are signed together; the public key is embedded in `volt.config.ts`
3. **SHA-256 hash check** - Downloaded bytes are hashed and compared against the signed manifest hash
4. **Semver downgrade prevention** - The new version must be greater than the current version. Downgrades are rejected to prevent rollback attacks

## Best Practices

1. **Declare minimal permissions** - Only request the capabilities your app actually needs
2. **Validate all IPC inputs** - Even though Volt guards against prototype pollution, validate handler arguments in your application code
3. **Use relative paths** - The `fs` module enforces this, but design your app to work with relative paths from the start
4. **Keep the public key secure** - The Ed25519 public key in `volt.config.ts` is safe to ship (it is public), but protect the corresponding private key used for signing
5. **Review CSP in production** - The default CSP is strict. If you need to relax it, understand the security implications
