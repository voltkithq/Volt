#!/usr/bin/env node

import prompts from 'prompts';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { updateVoltConfigContent } from './config-updater.js';
import { createProject } from './create-project.js';
import { escapeHtml, normalizeProjectName, ProjectOptions, toDisplayName } from './options.js';

interface PromptResponse {
  name?: string;
  framework?: ProjectOptions['framework'];
}

async function main(): Promise<void> {
  const argName = process.argv[2];

  console.log();
  console.log('  Volt - Lightweight Desktop App Framework');
  console.log();

  const response = await prompts(
    [
      {
        type: argName ? null : 'text',
        name: 'name',
        message: 'Project name:',
        initial: 'my-volt-app',
      },
      {
        type: 'select',
        name: 'framework',
        message: 'Select a framework:',
        choices: [
          { title: 'Vanilla', value: 'vanilla', description: 'Plain HTML/CSS/TypeScript' },
          { title: 'React', value: 'react', description: 'React 19 + TypeScript' },
          { title: 'Svelte', value: 'svelte', description: 'Svelte 5 + TypeScript' },
          { title: 'Vue', value: 'vue', description: 'Vue 3 + TypeScript' },
          { title: 'Enterprise', value: 'enterprise', description: 'Vanilla + enterprise-ready packaging defaults' },
        ],
      },
    ],
    {
      onCancel: () => {
        console.log('Cancelled.');
        process.exit(0);
      },
    },
  ) as PromptResponse;

  const projectName = normalizeProjectName(String(argName ?? response.name ?? ''));
  const options: ProjectOptions = {
    name: projectName,
    displayName: toDisplayName(projectName),
    framework: response.framework ?? 'vanilla',
  };

  await createProject(options);
}

export const __testOnly = {
  normalizeProjectName,
  toDisplayName,
  escapeHtml,
  updateVoltConfigContent,
  createProject,
};

const isEntrypoint = (() => {
  const argvPath = process.argv[1];
  if (!argvPath) {
    return false;
  }
  return resolve(argvPath) === fileURLToPath(import.meta.url);
})();

if (isEntrypoint) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
