import { existsSync } from 'node:fs';
import { createInterface } from 'node:readline/promises';
import { stdin as input, stdout as output } from 'node:process';
import { resolve } from 'node:path';
import type { Permission } from 'voltkit';
import { VALID_PERMISSIONS } from '../../utils/config/constants.js';
import { createPluginScaffold } from './scaffold.js';

export interface PluginInitOptions {
  cwd?: string;
}

interface PluginInitAnswers {
  pluginId: string;
  name: string;
  description: string;
  capabilities: Permission[];
}

export async function pluginInitCommand(
  projectName: string | undefined,
  options: PluginInitOptions = {},
  prompt: (projectName: string) => Promise<PluginInitAnswers> = promptForPluginAnswers,
): Promise<void> {
  const targetName = (projectName ?? '').trim();
  if (!targetName) {
    throw new Error('[volt:plugin] Project directory name is required.');
  }

  const targetDir = resolve(options.cwd ?? process.cwd(), targetName);
  if (existsSync(targetDir)) {
    throw new Error(`[volt:plugin] Target directory already exists: ${targetDir}`);
  }

  const answers = await prompt(targetName);
  createPluginScaffold({
    targetDir,
    ...answers,
  });

  console.log(`[volt:plugin] Created plugin scaffold in ${targetDir}`);
}

async function promptForPluginAnswers(projectName: string): Promise<PluginInitAnswers> {
  const rl = createInterface({ input, output });
  try {
    const pluginId = await ask(rl, 'Plugin ID (reverse-domain)', `com.example.${projectName}`);
    const name = await ask(rl, 'Plugin name', projectName);
    const description = await ask(rl, 'Description', `${name} plugin`);
    const capabilityAnswer = await ask(
      rl,
      `Capabilities (comma-separated from: ${VALID_PERMISSIONS.join(', ')})`,
      '',
    );
    return {
      pluginId,
      name,
      description,
      capabilities: parseCapabilities(capabilityAnswer),
    };
  } finally {
    rl.close();
  }
}

async function ask(
  rl: ReturnType<typeof createInterface>,
  label: string,
  fallback: string,
): Promise<string> {
  const answer = (await rl.question(`${label} [${fallback}]: `)).trim();
  return answer || fallback;
}

function parseCapabilities(raw: string): Permission[] {
  const values = raw
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean);
  const unique = new Set<Permission>();
  for (const value of values) {
    if (!VALID_PERMISSIONS.includes(value as Permission)) {
      throw new Error(`[volt:plugin] Unknown capability "${value}".`);
    }
    unique.add(value as Permission);
  }
  return [...unique];
}

export const __testOnly = {
  parseCapabilities,
  promptForPluginAnswers,
};
