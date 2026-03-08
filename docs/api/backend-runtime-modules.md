# Backend Runtime Modules

These modules are imported from `src/backend.ts` (Boa runtime), not from renderer code.

```ts
import * as voltDb from 'volt:db';
import * as voltHttp from 'volt:http';
import * as voltSecureStorage from 'volt:secureStorage';
```

## Permissions

Declare required permissions in `volt.config.ts`:

```ts
import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'My App',
  permissions: ['db', 'http', 'secureStorage'],
});
```

| Module | Permission |
|--------|------------|
| `volt:db` | `'db'` |
| `volt:http` | `'http'` |
| `volt:secureStorage` | `'secureStorage'` |

## `volt:db`

Embedded SQLite access from backend code.

### API Surface

| Method | Signature | Notes |
|--------|-----------|-------|
| `open` | `open(path: string): Promise<void>` | Must be called before query/execute calls. |
| `close` | `close(): Promise<void>` | Safe to call multiple times. |
| `execute` | `execute(sql: string, params?: unknown[]): Promise<{ rowsAffected: number }>` | For inserts/updates/deletes and DDL. |
| `query` | `query(sql: string, params?: unknown[]): Promise<unknown[]>` | Returns array rows. |
| `queryOne` | `queryOne(sql: string, params?: unknown[]): Promise<unknown \| null>` | Returns first row or `null`. |
| `transaction` | `transaction<T>(callback: () => Promise<T> \| T): Promise<T>` | Wraps callback in a SQL transaction. |

### Example

```ts
import * as voltDb from 'volt:db';

await voltDb.open('app/data.sqlite');
await voltDb.execute(
  'CREATE TABLE IF NOT EXISTS todos (id TEXT PRIMARY KEY, title TEXT NOT NULL)',
);
await voltDb.execute('INSERT INTO todos (id, title) VALUES (?, ?)', ['1', 'Ship beta']);
const rows = await voltDb.query('SELECT id, title FROM todos ORDER BY title ASC');
await voltDb.close();
```

## `volt:http`

Backend HTTP fetch with permission checks.

### API Surface

| Method | Signature | Notes |
|--------|-----------|-------|
| `fetch` | `fetch(request: { url: string; method?: string; headers?: Record<string, string>; body?: unknown; timeoutMs?: number }): Promise<{ status: number; headers: Record<string, string[]>; text(): Promise<string>; json(): Promise<unknown>; }>` | Enforces permission checks, uses a 30s request timeout, and caps response body reads at 2 MiB. |

### Example

```ts
import * as voltHttp from 'volt:http';

const response = await voltHttp.fetch({
  url: 'https://api.example.com/health',
  method: 'GET',
  timeoutMs: 10_000,
});

if (response.status !== 200) {
  throw new Error(`Health check failed: ${response.status}`);
}

const body = await response.json();
```

## `volt:secureStorage`

Key-value secret storage for backend code.

### API Surface

| Method | Signature |
|--------|-----------|
| `set` | `set(key: string, value: string): Promise<void>` |
| `get` | `get(key: string): Promise<string \| null>` |
| `has` | `has(key: string): Promise<boolean>` |
| `delete` | `delete(key: string): Promise<void>` |

### Key Validation Rules

- key must be a non-empty string after trim
- key length must be at most 256 characters

### Example

```ts
import * as voltSecureStorage from 'volt:secureStorage';

const tokenKey = 'auth/access-token';
await voltSecureStorage.set(tokenKey, 'secret-value');

const hasToken = await voltSecureStorage.has(tokenKey);
const token = await voltSecureStorage.get(tokenKey);

if (hasToken && token) {
  console.log('token loaded');
}
```

## Error Handling Pattern

```ts
try {
  const rows = await voltDb.query('SELECT 1');
  console.log(rows);
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[backend] runtime module failure: ${message}`);
}
```
