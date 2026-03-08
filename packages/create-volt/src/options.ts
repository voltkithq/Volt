import { isAbsolute } from 'node:path';

export interface ProjectOptions {
  name: string;
  displayName: string;
  framework: 'vanilla' | 'react' | 'svelte' | 'vue' | 'enterprise';
}

const PROJECT_NAME_RE = /^[a-z0-9](?:[a-z0-9._-]*[a-z0-9])?$/;

export function normalizeProjectName(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) {
    throw new Error('Project name cannot be empty.');
  }
  if (trimmed === '.' || trimmed === '..') {
    throw new Error('Project name cannot be "." or "..".');
  }
  if (isAbsolute(trimmed) || trimmed.includes('/') || trimmed.includes('\\')) {
    throw new Error('Project name must be a single directory segment, not a path.');
  }

  const normalized = trimmed.toLowerCase().replace(/\s+/g, '-');
  if (!PROJECT_NAME_RE.test(normalized)) {
    throw new Error(
      'Project name must be lowercase and use letters, numbers, ".", "_" or "-".',
    );
  }

  return normalized;
}

export function toDisplayName(projectName: string): string {
  const parts = projectName.split(/[-_]+/).filter(Boolean);
  if (parts.length === 0) {
    return 'Volt App';
  }
  return parts.map((part) => part[0].toUpperCase() + part.slice(1)).join(' ');
}

export function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}
