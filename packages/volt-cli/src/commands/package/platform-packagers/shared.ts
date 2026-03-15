import { copyFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

export const SIDECAR_FILES = ['volt-assets.bin', 'volt-backend.js', 'volt-config.json'] as const;

export function copySidecarFiles(sourceDir: string, targetDir: string): void {
  for (const sidecar of SIDECAR_FILES) {
    const srcPath = resolve(sourceDir, sidecar);
    if (existsSync(srcPath)) {
      copyFileSync(srcPath, resolve(targetDir, sidecar));
    }
  }
}
