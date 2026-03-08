import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const repoRoot = resolve(__dirname, '..', '..');
const voltCliEntry = resolve(repoRoot, 'packages/volt-cli/dist/index.js');
const testConfigPath = resolve(repoRoot, 'volt.test.config.mjs');
const artifactDir = resolve(repoRoot, 'artifacts', 'e2e-smoke', process.platform);

function assertFile(filePath, description) {
  if (!existsSync(filePath)) {
    throw new Error(`[smoke] Missing ${description}: ${filePath}`);
  }
}

function main() {
  assertFile(voltCliEntry, 'volt-cli build output');
  assertFile(testConfigPath, 'volt test config');

  const retries = process.env['VOLT_E2E_RETRIES'] ?? '1';

  execFileSync('node', [
    voltCliEntry,
    'test',
    '--config',
    testConfigPath,
    '--retries',
    retries,
    '--artifacts-dir',
    artifactDir,
  ], {
    cwd: repoRoot,
    stdio: 'inherit',
    env: process.env,
  });

  assertFile(join(artifactDir, 'run-summary.json'), 'e2e run summary');
}

main();
