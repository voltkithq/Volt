import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { IPC_DEMO_FRONTEND_SMOKE_SOURCE, IPC_DEMO_SMOKE_CONFIG_SOURCE } from './sources.js';

export const RESULT_FILE = '.volt-smoke-result.json';

export function prepareIpcDemoSmokeProject(projectDir: string): void {
  const backendPath = join(projectDir, 'src', 'backend.ts');
  const backendSource = readFileSync(backendPath, 'utf8');
  writeFileSync(backendPath, patchBackendForSmoke(backendSource), 'utf8');

  const tsConfigPath = join(projectDir, 'volt.config.ts');
  rmSync(tsConfigPath, { force: true });
  writeFileSync(join(projectDir, 'volt.config.mjs'), IPC_DEMO_SMOKE_CONFIG_SOURCE, 'utf8');

  const mainPath = join(projectDir, 'src', 'main.ts');
  writeFileSync(mainPath, IPC_DEMO_FRONTEND_SMOKE_SOURCE, 'utf8');
}

function patchBackendForSmoke(source: string): string {
  let next = source;
  const fsImport = "import * as voltFs from 'volt:fs';";
  if (!next.includes(fsImport)) {
    const anchorImport = "import * as voltEvents from 'volt:events';";
    if (!next.includes(anchorImport)) {
      throw new Error(
        '[volt:test] failed to patch ipc-demo backend: expected voltEvents import anchor.',
      );
    }
    next = next.replace(anchorImport, `${anchorImport}\n${fsImport}`);
  }

  if (next.includes("ipcMain.handle('smoke:complete'")) {
    return next;
  }

  return `${next.trimEnd()}

const IPC_DEMO_SMOKE_RESULT_FILE = '.volt-smoke-result.json';

ipcMain.handle('smoke:complete', async (payload: unknown) => {
  await voltFs.writeFile(IPC_DEMO_SMOKE_RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
`;
}
