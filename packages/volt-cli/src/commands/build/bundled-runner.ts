import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function resolveBundledRunnerCrates(): string | null {
  const cliDistDir = dirname(fileURLToPath(import.meta.url));
  const bundledPath = resolve(cliDistDir, '..', '..', '..', 'runner-crates');
  return existsSync(resolve(bundledPath, 'Cargo.toml')) ? bundledPath : null;
}
