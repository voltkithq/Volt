import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

export interface TempProject {
  rootDir: string;
  cleanup: () => void;
}

export function createTempProject(files: Record<string, string>): TempProject {
  const rootDir = mkdtempSync(join(process.cwd(), '.tmp-dev-backend-test-'));
  for (const [relativePath, contents] of Object.entries(files)) {
    const absolutePath = join(rootDir, relativePath);
    mkdirSync(dirname(absolutePath), { recursive: true });
    writeFileSync(absolutePath, contents, 'utf8');
  }
  return {
    rootDir,
    cleanup: () => {
      rmSync(rootDir, { recursive: true, force: true });
    },
  };
}
