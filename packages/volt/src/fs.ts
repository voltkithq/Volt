/**
 * Sandboxed file system module.
 * All paths are relative to the application's allowed scope.
 * No absolute paths are accepted from the frontend.
 * API methods return Promises for compatibility, but native fs operations are synchronous
 * and execute on the calling thread.
 * Requires `permissions: ['fs']` in volt.config.ts.
 */

import { bindScope } from './fs/scoped.js';
import {
  readDir,
  readFile,
  readFileBinary,
  writeFile,
  writeFileBinary,
  stat,
  exists,
  mkdir,
  remove,
} from './fs/root.js';
import { getBaseDir } from './fs/state.js';
import { validatePath } from './fs/validation.js';
import { createWatcher } from './fs/watcher.js';

export type { FileInfo, FileWatcher, ScopedFs, WatchEvent, WatchOptions } from './fs/types.js';
export { setBaseDir } from './fs/state.js';

/**
 * Watch a directory for file changes within the app scope.
 *
 * @example
 * ```ts
 * const watcher = await fs.watch('data', { recursive: true });
 * const events = await watcher.poll();
 * await watcher.close();
 * ```
 */
async function watch(path: string, options?: import('./fs/types.js').WatchOptions) {
  validatePath(path);
  return createWatcher(getBaseDir(), path, options);
}

/** Sandboxed file system APIs. Requires `permissions: ['fs']` in volt.config.ts. */
export const fs = {
  readFile,
  readFileBinary,
  writeFile,
  writeFileBinary,
  readDir,
  stat,
  exists,
  mkdir,
  remove,
  bindScope,
  watch,
};
