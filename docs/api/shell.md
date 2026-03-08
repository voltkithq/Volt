# Shell

Secure URL opening. Requires `permissions: ['shell']`.

## `shell.openExternal(url): Promise<void>`

Open a URL in the default system application (browser, email client, etc.).

```ts
import { shell } from 'voltkit';

await shell.openExternal('https://example.com');
await shell.openExternal('mailto:hello@example.com');
```

**Parameters:**
- `url: string` — The URL to open

**Throws:**
- If the URL is invalid
- If the protocol is not allowed

### Allowed Protocols

| Protocol | Allowed | Example |
|----------|---------|---------|
| `https:` | Yes | `https://example.com` |
| `http:` | Yes | `http://localhost:3000` |
| `mailto:` | Yes | `mailto:user@example.com` |
| `file:` | **No** | `file:///etc/passwd` |
| `javascript:` | **No** | `javascript:alert(1)` |
| `data:` | **No** | `data:text/html,...` |
| `ftp:` | **No** | `ftp://files.example.com` |
| `vbscript:` | **No** | `vbscript:msgbox` |
| All others | **No** | — |

### Security

URL validation happens at two layers:

1. **TypeScript** — Uses the `URL` constructor to parse and check `protocol`
2. **Rust** — Uses the `url` crate to validate the scheme

Both layers must pass for the URL to be opened. The native call is never made if validation fails.
