import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const ignoreConfigPath = resolve(scriptDir, 'cargo-audit-ignore.json');
const ignoreConfig = JSON.parse(readFileSync(ignoreConfigPath, 'utf8'));

const ignoredIds = [];
const expiredEntries = [];
const rawEntries = Array.isArray(ignoreConfig.ignore) ? ignoreConfig.ignore : [];
const today = new Date().toISOString().slice(0, 10);

for (const entry of rawEntries) {
  if (typeof entry === 'string') {
    ignoredIds.push(entry);
    continue;
  }

  if (entry && typeof entry === 'object' && typeof entry.id === 'string') {
    ignoredIds.push(entry.id);
    const reviewBy = entry.reviewBy;
    if (typeof reviewBy === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(reviewBy) && reviewBy < today) {
      expiredEntries.push(entry);
    }
    continue;
  }

  console.error(`[ci] Invalid cargo-audit ignore entry in ${ignoreConfigPath}: ${JSON.stringify(entry)}`);
  process.exit(1);
}

if (expiredEntries.length > 0) {
  console.error('[ci] Expired cargo-audit ignore entries detected:');
  for (const entry of expiredEntries) {
    console.error(
      `  - ${entry.id} (reviewBy=${entry.reviewBy}, owner=${entry.owner ?? 'unassigned'})`,
    );
  }
  process.exit(1);
}

const args = ['audit', '--deny', 'warnings'];
for (const advisoryId of ignoredIds) {
  args.push('--ignore', advisoryId);
}

const result = spawnSync('cargo', args, { stdio: 'inherit' });
if (result.error) {
  console.error(`[ci] Failed to execute cargo audit: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
