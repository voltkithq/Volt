import path from 'node:path';

const KIND_BY_EXTENSION = new Map([
  ['.rs', 'Rust'],
  ['.ts', 'TypeScript'],
  ['.tsx', 'TypeScript (React)'],
  ['.mts', 'TypeScript (ESM)'],
  ['.cts', 'TypeScript (CJS)'],
  ['.js', 'JavaScript'],
  ['.jsx', 'JavaScript (React)'],
  ['.mjs', 'JavaScript (ESM)'],
  ['.cjs', 'JavaScript (CJS)'],
  ['.json', 'JSON'],
  ['.toml', 'TOML'],
  ['.yaml', 'YAML'],
  ['.yml', 'YAML'],
  ['.md', 'Markdown'],
  ['.css', 'CSS'],
  ['.html', 'HTML'],
  ['.svg', 'SVG'],
  ['.lock', 'Lockfile'],
  ['.ps1', 'PowerShell'],
  ['.sh', 'Shell'],
]);

export const TEXT_EXTENSIONS = new Set([
  '.md',
  '.ts',
  '.tsx',
  '.mts',
  '.cts',
  '.js',
  '.jsx',
  '.mjs',
  '.cjs',
  '.json',
  '.toml',
  '.yaml',
  '.yml',
  '.rs',
  '.sh',
  '.ps1',
  '.txt',
  '.lock',
  '.css',
  '.html',
  '.svg',
]);

export const TOP_LEVEL_PURPOSE = new Map([
  ['(root)', 'Monorepo orchestration, workspace policy, and shared metadata.'],
  ['.github', 'Issue templates and CI/CD workflow automation.'],
  ['crates', 'Rust runtime, native bridge, and core desktop platform crates.'],
  ['packages', 'TypeScript framework, CLI, and scaffolding packages.'],
  ['examples', 'Reference applications used for feature validation and demos.'],
  ['docs', 'User-facing and contributor-facing documentation.'],
  ['scripts', 'Automation for CI, audits, and maintenance workflows.'],
  ['contracts', 'Compatibility and schema contract references.'],
]);

export const GROUP_PURPOSE = new Map([
  ['crates/volt-core', 'Core Rust desktop runtime primitives (windowing, IPC, permissions, updater).'],
  ['crates/volt-runner', 'Standalone runner binary hosting QuickJS runtime, modules, and IPC bridge.'],
  ['crates/volt-updater-helper', 'Windows helper binary for safe in-place updates and rollback.'],
  ['crates/volt-napi', 'Node/N-API bridge used by development runtime.'],
  ['packages/volt', 'Framework API surface and TypeScript types consumed by apps.'],
  ['packages/volt-cli', 'Build/dev/package commands and runtime orchestration tooling.'],
  ['packages/create-volt', 'Project scaffolding package for new Volt apps.'],
  ['examples/hello-world', 'Minimal starter example for fast sanity checks.'],
  ['examples/ipc-demo', 'End-to-end IPC and native-module behavior showcase.'],
  ['examples/todo-app', 'Stateful app example with frontend/backend workflow.'],
  ['docs/api', 'API reference pages grouped by module.'],
  ['scripts/ci', 'CI scripts for validation, audits, and policy enforcement.'],
  ['scripts/soak', 'Long-running soak test orchestration and reporting scripts.'],
  ['scripts/contracts', 'Contract compatibility checks.'],
  ['scripts/docs', 'Documentation generation and maintenance scripts.'],
  ['contracts/annotations', 'Contract annotation and compatibility docs.'],
  ['.github/workflows', 'GitHub Actions pipelines for CI, releases, and nightly checks.'],
  ['.github/ISSUE_TEMPLATE', 'Issue templates for bug reports and feature requests.'],
  ['.github', 'Repository-level GitHub configuration and templates.'],
  ['docs', 'High-level architecture, configuration, and operational documentation.'],
  ['scripts', 'Shared repository automation scripts.'],
]);

export const EXCLUDED_PATH_PARTS = new Set([
  '.git',
  'node_modules',
  'target',
  '.turbo',
  'dist',
  'build',
  'coverage',
  '.volt-dev',
  '.volt-build',
  'dist-volt',
]);

export function topLevelKey(filePath) {
  const parts = filePath.split('/');
  return parts.length === 1 ? '(root)' : parts[0];
}

export function groupKey(filePath) {
  const parts = filePath.split('/');
  if (parts.length === 1) {
    return '(root)';
  }

  const first = parts[0];
  if (['crates', 'packages', 'examples', 'scripts', 'docs', 'contracts'].includes(first)) {
    if (parts.length >= 3) {
      return `${first}/${parts[1]}`;
    }
    return first;
  }

  return first;
}

export function isTestFile(filePath, content) {
  const lower = filePath.toLowerCase();
  if (
    lower.includes('/test/') ||
    lower.includes('/tests/') ||
    lower.endsWith('.test.ts') ||
    lower.endsWith('.test.tsx') ||
    lower.endsWith('.test.js') ||
    lower.endsWith('.test.mjs')
  ) {
    return true;
  }
  if (!content) {
    return false;
  }
  return content.includes('describe(') && content.includes('it(');
}

function markdownSummary(filePath, content) {
  if (!content) {
    return fallbackSummary(filePath);
  }

  for (const rawLine of content.split(/\r?\n/u)) {
    const line = rawLine.trim();
    if (line.startsWith('#')) {
      return `Documentation for ${line.replace(/^#+\s*/u, '')}.`;
    }
  }

  return fallbackSummary(filePath);
}

function docCommentSummary(content) {
  if (!content) {
    return null;
  }

  const header = content.split(/\r?\n/u).slice(0, 80).join('\n');
  const patterns = [
    /^\s*\/\/!\s+(.+)$/mu,
    /^\s*\/\/\/\s+(.+)$/mu,
    /^\s*\/\*\*\s*\n?\s*\*\s+(.+)$/mu,
  ];

  for (const pattern of patterns) {
    const match = header.match(pattern);
    if (match && match[1]) {
      return normalizeSentence(match[1]);
    }
  }

  return null;
}

export function fallbackSummary(filePath) {
  const base = path.basename(filePath);
  const ext = path.extname(base);
  const stem = base.slice(0, base.length - ext.length);

  if (base === 'Cargo.toml') {
    return 'Rust crate manifest and dependency configuration.';
  }
  if (base === 'package.json') {
    return 'Node package manifest, scripts, and dependency constraints.';
  }
  if (base === 'tsconfig.json') {
    return 'TypeScript compiler and project build configuration.';
  }
  if (base === 'README.md') {
    return 'Module usage and contributor guidance.';
  }
  if (base === 'main.rs') {
    return 'Executable entrypoint and runtime bootstrap.';
  }
  if (base === 'lib.rs') {
    return 'Library entrypoint and module exports.';
  }
  if (base === 'mod.rs') {
    return 'Module declarations and re-exports.';
  }
  if (base === 'build.rs') {
    const parts = filePath.split('/');
    if (parts.length === 3 && parts[0] === 'crates') {
      return 'Rust build-time embedding and compile hooks.';
    }
    return 'Implementation for build-specific module behavior.';
  }

  return `Implementation for ${stem.replace(/[-_.]+/gu, ' ')}.`;
}

export function fileSummary(filePath, content) {
  const ext = path.extname(filePath).toLowerCase();
  if (isTestFile(filePath, content)) {
    return 'Automated tests for runtime behavior and regressions.';
  }
  if (ext === '.md') {
    return markdownSummary(filePath, content);
  }
  return docCommentSummary(content) ?? fallbackSummary(filePath);
}

export function fileKind(filePath, content) {
  const ext = path.extname(filePath).toLowerCase();
  if (isTestFile(filePath, content)) {
    return 'Test';
  }
  return KIND_BY_EXTENSION.get(ext) ?? 'Asset/Other';
}

export function normalizeSentence(text) {
  const clean = text
    .replace(/[^\x20-\x7E]/gu, ' ')
    .replace(/[`*_]/gu, '')
    .replace(/\s+/gu, ' ')
    .trim();

  if (clean.length === 0) {
    return 'Implementation details in this file.';
  }

  if (/[.!?]$/u.test(clean)) {
    return clean;
  }

  return `${clean}.`;
}

export function escapePipe(value) {
  return String(value).replace(/\|/gu, '\\|');
}

export function sortGroups(a, b) {
  const score = (key) => {
    if (key === '(root)') return 0;
    if (key.startsWith('crates/')) return 1;
    if (key.startsWith('packages/')) return 2;
    if (key.startsWith('examples/')) return 3;
    if (key.startsWith('docs/')) return 4;
    if (key === 'docs') return 5;
    if (key.startsWith('scripts/')) return 6;
    if (key === 'scripts') return 7;
    if (key.startsWith('contracts/')) return 8;
    if (key === 'contracts') return 9;
    return 10;
  };

  const scoreDiff = score(a) - score(b);
  if (scoreDiff !== 0) {
    return scoreDiff;
  }

  return a.localeCompare(b);
}
