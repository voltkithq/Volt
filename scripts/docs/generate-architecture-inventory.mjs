#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  EXCLUDED_PATH_PARTS,
  GROUP_PURPOSE,
  TEXT_EXTENSIONS,
  TOP_LEVEL_PURPOSE,
  escapePipe,
  fileKind,
  fileSummary,
  groupKey,
  sortGroups,
  topLevelKey,
} from './architecture-inventory.shared.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..', '..');
const outputPath = path.join(repoRoot, 'docs', 'architecture-inventory.md');

function isExcludedPath(filePath) {
  const parts = filePath.split('/');
  return parts.some((part) => EXCLUDED_PATH_PARTS.has(part));
}

function isLikelyText(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (TEXT_EXTENSIONS.has(ext)) {
    return true;
  }

  const base = path.basename(filePath);
  return (
    base === 'Cargo.lock' ||
    base === 'Cargo.toml' ||
    base === 'package.json' ||
    base === 'tsconfig.json' ||
    base.startsWith('README')
  );
}

function getTrackedFiles() {
  const output = execFileSync('git', ['ls-files'], {
    cwd: repoRoot,
    encoding: 'utf8',
  });

  return output
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .filter((line) => line !== 'docs/architecture-inventory.md')
    .filter((line) => !isExcludedPath(line));
}

function readTextFile(filePath) {
  const absolute = path.join(repoRoot, filePath);
  try {
    const content = readFileSync(absolute, 'utf8');
    if (content.includes('\u0000')) {
      return null;
    }
    return content;
  } catch {
    return null;
  }
}

function countNonEmptyLines(content) {
  if (!content) {
    return null;
  }

  let count = 0;
  for (const line of content.split(/\r?\n/u)) {
    if (line.trim().length > 0) {
      count += 1;
    }
  }
  return count;
}

function buildFileRecords() {
  const trackedFiles = getTrackedFiles();
  return trackedFiles.map((filePath) => {
    const content = isLikelyText(filePath) ? readTextFile(filePath) : null;
    return {
      path: filePath,
      kind: fileKind(filePath, content),
      loc: countNonEmptyLines(content),
      summary: fileSummary(filePath, content),
    };
  });
}

function renderTopLevelSummary(lines, files) {
  const topLevelStats = new Map();
  for (const file of files) {
    const key = topLevelKey(file.path);
    topLevelStats.set(key, (topLevelStats.get(key) ?? 0) + 1);
  }

  lines.push('## Top-Level Folders');
  lines.push('');
  lines.push('| Folder | Files | What It Contains |');
  lines.push('| --- | ---: | --- |');

  for (const [folder, count] of [...topLevelStats.entries()].sort((a, b) =>
    a[0].localeCompare(b[0]),
  )) {
    const purpose =
      TOP_LEVEL_PURPOSE.get(folder) ?? 'Project files for this repository area.';
    lines.push(
      `| \`${escapePipe(folder)}\` | ${count} | ${escapePipe(purpose)} |`,
    );
  }
  lines.push('');
}

function renderGroupCatalog(lines, files) {
  const groups = new Map();
  for (const file of files) {
    const key = groupKey(file.path);
    if (!groups.has(key)) {
      groups.set(key, []);
    }
    groups.get(key).push(file);
  }

  lines.push('## Folder And File Catalog');
  lines.push('');

  for (const group of [...groups.keys()].sort(sortGroups)) {
    const groupFiles = groups.get(group) ?? [];
    groupFiles.sort((a, b) => a.path.localeCompare(b.path));

    const purpose =
      GROUP_PURPOSE.get(group) ??
      `Contains files under \`${group}\` for this subsystem.`;

    lines.push(`### \`${group}\``);
    lines.push('');
    lines.push(`- Purpose: ${purpose}`);
    lines.push(`- Files: ${groupFiles.length}`);
    lines.push('');
    lines.push('| File | Kind | LOC* | What It Has |');
    lines.push('| --- | --- | ---: | --- |');

    for (const file of groupFiles) {
      const loc = file.loc ?? '-';
      lines.push(
        `| \`${escapePipe(file.path)}\` | ${escapePipe(file.kind)} | ${loc} | ${escapePipe(file.summary)} |`,
      );
    }
    lines.push('');
  }
}

function renderMarkdown(files) {
  const generatedAt = new Date().toISOString();
  const readableCount = files.filter((item) => item.loc !== null).length;
  const lines = [];

  lines.push('# Architecture Inventory');
  lines.push('');
  lines.push('This document is generated from tracked repository files.');
  lines.push('Do not edit manually.');
  lines.push('');
  lines.push(`Generated at: \`${generatedAt}\``);
  lines.push('');
  lines.push('## How To Refresh');
  lines.push('');
  lines.push('Run:');
  lines.push('');
  lines.push('```bash');
  lines.push('pnpm docs:architecture');
  lines.push('```');
  lines.push('');
  lines.push('## Repository Summary');
  lines.push('');
  lines.push(`- Total tracked files: **${files.length}**`);
  lines.push(`- Text-readable files: **${readableCount}**`);
  lines.push('');

  renderTopLevelSummary(lines, files);
  renderGroupCatalog(lines, files);

  lines.push('*LOC counts non-empty lines for text-readable files.');
  lines.push('');

  return `${lines.join('\n')}\n`;
}

function main() {
  const files = buildFileRecords();
  const markdown = renderMarkdown(files);

  mkdirSync(path.dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, markdown, 'utf8');

  console.log(
    `[docs] Architecture inventory generated: ${path.relative(repoRoot, outputPath)} (${files.length} files)`,
  );
}

main();
