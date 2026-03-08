import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectDir = dirname(scriptDir);

function readProjectFile(relativePath) {
  const path = join(projectDir, relativePath);
  return readFileSync(path, 'utf8');
}

function assertContains(relativePath, expected) {
  const content = readProjectFile(relativePath);
  if (!content.includes(expected)) {
    throw new Error(`Missing "${expected}" in ${relativePath}`);
  }
}

function runPreflightChecks() {
  assertContains('volt.config.ts', "'secureStorage'");
  assertContains('src/backend.ts', "ipcMain.handle('secure-storage:set'");
  assertContains('src/backend.ts', "ipcMain.handle('secure-storage:get'");
  assertContains('src/main.ts', "invoke<SecureStorageSetResponse>('secure-storage:set'");
  assertContains('index.html', 'btn-secret-set');
}

function printManualChecklist() {
  const lines = [
    '',
    'Manual verification checklist (secureStorage flow):',
    '1. Run `pnpm --filter ipc-demo build` from repo root.',
    '2. Launch the demo with `pnpm --filter ipc-demo dev`.',
    '3. In the Secure Storage panel, keep key `ipc-demo/demo-secret`.',
    '4. Enter a secret value and click `Set`; expect `{ "ok": true, "has": true }`.',
    '5. Click `Has`; expect `{ "has": true }`.',
    '6. Click `Get`; expect the stored `value` string.',
    '7. Click `Delete`; expect `{ "ok": true, "has": false }`.',
    '8. Click `Has` again; expect `{ "has": false }`.',
    '9. Confirm event log includes `demo:secure-storage-updated` entries.',
    '',
  ];
  console.log(lines.join('\n'));
}

try {
  runPreflightChecks();
  console.log('[secure-storage manual] Preflight checks passed.');
  printManualChecklist();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`[secure-storage manual] ${message}`);
  process.exit(1);
}
