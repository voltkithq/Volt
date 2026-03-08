# File System

Sandboxed file operations. Requires `permissions: ['fs']`.

All paths must be **relative** to the application's scope. Absolute paths, path traversal (`..`), and Windows reserved device names are rejected.

## `fs.readFile(path): Promise<string>`

Read a file as a UTF-8 string.

```ts
import { fs } from 'voltkit';

const content = await fs.readFile('data/config.json');
```

## `fs.readFileBinary(path): Promise<Uint8Array>`

Read a file as raw bytes.

```ts
const bytes = await fs.readFileBinary('assets/image.png');
```

## `fs.writeFile(path, data): Promise<void>`

Write a UTF-8 string to a file. Creates the file if it doesn't exist.

```ts
await fs.writeFile('data/output.json', JSON.stringify({ key: 'value' }));
```

## `fs.writeFileBinary(path, data): Promise<void>`

Write raw bytes to a file.

```ts
await fs.writeFileBinary('data/binary.dat', new Uint8Array([0x00, 0xFF]));
```

## `fs.readDir(path): Promise<string[]>`

List entries in a directory. Returns file and directory names.

```ts
const entries = await fs.readDir('data');
// ['config.json', 'output.json', 'subdir']
```

## `fs.stat(path): Promise<FileInfo>`

Get metadata for a file or directory.

```ts
const info = await fs.stat('data/config.json');
console.log(info.size);     // 1024
console.log(info.isFile);   // true
console.log(info.isDir);    // false
console.log(info.readonly); // false
```

## `fs.mkdir(path): Promise<void>`

Create a directory. Parent directories are created if needed.

```ts
await fs.mkdir('data/nested/deep');
```

## `fs.remove(path): Promise<void>`

Remove a file or directory (recursive for directories).

```ts
await fs.remove('data/old-file.txt');
```

## `FileInfo`

```ts
interface FileInfo {
  size: number;      // File size in bytes
  isFile: boolean;   // Whether the path is a file
  isDir: boolean;    // Whether the path is a directory
  readonly: boolean; // Whether the file is read-only
}
```

## Security

Paths are validated at two layers:

1. **TypeScript layer** — Rejects absolute paths (`/`, `\`, drive letters) and `..` traversal
2. **Rust layer** — `safe_resolve()` canonicalizes both the base and resolved paths, then verifies the result is under the base directory. Also blocks Windows reserved device names (`CON`, `NUL`, `COM1`, etc.)
