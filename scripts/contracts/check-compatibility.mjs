import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';

const CONTRACT_FILE_PATTERN = /^contracts\/.+\.json$/;
const ANNOTATION_FILE_PATTERN = /^contracts\/annotations\/.+\.md$/;
const ANNOTATION_README = 'contracts/annotations/README.md';

function main() {
  const changedFiles = getChangedFiles();
  if (changedFiles.length === 0) {
    console.log('[contract-compat] no changed files detected; gate passes.');
    return;
  }

  const changedContracts = changedFiles.filter((file) => CONTRACT_FILE_PATTERN.test(file));
  if (changedContracts.length === 0) {
    console.log('[contract-compat] no contract fixture changes detected; gate passes.');
    return;
  }

  const changedAnnotations = changedFiles.filter(
    (file) => ANNOTATION_FILE_PATTERN.test(file) && file !== ANNOTATION_README,
  );
  const changelogChanged = changedFiles.includes('CHANGELOG.md');
  const policyChanged = changedFiles.includes('docs/compatibility-policy.md');

  const failures = [];

  if (!changelogChanged) {
    failures.push(
      'Contract fixture changed but CHANGELOG.md was not updated. Add a compatibility note entry.',
    );
  }

  if (changedAnnotations.length === 0) {
    failures.push(
      'Contract fixture changed but no annotation was updated under contracts/annotations/.',
    );
  }

  const parsedAnnotations = changedAnnotations.map((file) => parseAnnotation(file, failures));
  const validAnnotations = parsedAnnotations.filter((value) => value != null);
  const annotatedContracts = new Set(validAnnotations.map((annotation) => annotation.contract));
  const uncoveredContracts = changedContracts.filter((contract) => !annotatedContracts.has(contract));
  if (uncoveredContracts.length > 0) {
    failures.push(
      `Missing annotation coverage for changed contract files: ${uncoveredContracts.join(', ')}`,
    );
  }

  const hasBreakingChange = validAnnotations.some((annotation) => annotation.changeType === 'breaking');
  if (hasBreakingChange && !policyChanged) {
    failures.push(
      'Breaking contract annotation requires docs/compatibility-policy.md update in the same change.',
    );
  }

  if (failures.length > 0) {
    console.error('[contract-compat] gate failed:');
    for (const failure of failures) {
      console.error(`- ${failure}`);
    }
    process.exit(1);
  }

  console.log('[contract-compat] gate passed.');
  console.log(`[contract-compat] changed contracts: ${changedContracts.join(', ')}`);
}

function parseAnnotation(file, failures) {
  if (!existsSync(file)) {
    failures.push(`Annotation file is missing from workspace: ${file}`);
    return null;
  }

  const contents = readFileSync(file, 'utf8');
  const contract = matchField(contents, 'Contract');
  const changeType = matchField(contents, 'Change-Type')?.toLowerCase();
  const changelogUpdated = matchField(contents, 'Changelog-Updated')?.toLowerCase();

  if (!contract) {
    failures.push(`Annotation missing "Contract" field: ${file}`);
    return null;
  }
  if (!CONTRACT_FILE_PATTERN.test(contract)) {
    failures.push(`Annotation "Contract" must point to contracts/*.json: ${file}`);
    return null;
  }
  if (!changeType || !['backward-compatible', 'breaking', 'internal'].includes(changeType)) {
    failures.push(
      `Annotation "Change-Type" must be backward-compatible|breaking|internal: ${file}`,
    );
    return null;
  }
  if (changelogUpdated !== 'yes') {
    failures.push(`Annotation "Changelog-Updated" must be "yes": ${file}`);
    return null;
  }

  return { file, contract, changeType };
}

function matchField(contents, key) {
  const pattern = new RegExp(`^-\\s*${escapeRegex(key)}\\s*:\\s*(.+)$`, 'im');
  const match = contents.match(pattern);
  return match?.[1]?.trim() ?? null;
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function getChangedFiles() {
  const override = parseChangedFileOverride(process.env.CONTRACT_CHANGED_FILES);
  if (override != null) {
    console.log('[contract-compat] using CONTRACT_CHANGED_FILES override.');
    return override;
  }

  if (!isGitRepo()) {
    console.warn('[contract-compat] not a git repository and no CONTRACT_CHANGED_FILES override set.');
    return [];
  }

  const range = resolveDiffRange();
  console.log(`[contract-compat] using git diff range: ${range}`);
  const output = git(['diff', '--name-only', range]);
  return output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function parseChangedFileOverride(raw) {
  if (!raw || raw.trim() === '') {
    return null;
  }

  return raw
    .split(/\r?\n|,/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function resolveDiffRange() {
  const explicit = process.env.CONTRACT_DIFF_RANGE;
  if (explicit && explicit.trim() !== '') {
    return explicit.trim();
  }

  const baseRef = process.env.GITHUB_BASE_REF;
  if (baseRef && baseRef.trim() !== '') {
    git(['fetch', '--no-tags', '--depth=1', 'origin', baseRef.trim()]);
    return `origin/${baseRef.trim()}...HEAD`;
  }

  const before = process.env.GITHUB_EVENT_BEFORE;
  if (before && before.trim() !== '' && !/^0+$/.test(before.trim())) {
    return `${before.trim()}...HEAD`;
  }

  try {
    git(['rev-parse', '--verify', 'HEAD~1']);
    console.warn(
      '[contract-compat] falling back to HEAD~1...HEAD; set CONTRACT_DIFF_RANGE for full branch checks.',
    );
    return 'HEAD~1...HEAD';
  } catch {
    return 'HEAD';
  }
}

function isGitRepo() {
  try {
    return git(['rev-parse', '--is-inside-work-tree']).trim() === 'true';
  } catch {
    return false;
  }
}

function git(args) {
  return execFileSync('git', args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

main();
