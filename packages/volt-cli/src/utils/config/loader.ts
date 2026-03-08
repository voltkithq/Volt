import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import type { VoltConfig } from 'voltkit';
import { CONFIG_FILES, DEFAULT_CONFIG } from './constants.js';
import type { LoadConfigOptions } from './types.js';
import { validateConfig } from './validator.js';

export async function loadConfig(projectRoot: string, options: LoadConfigOptions = {}): Promise<VoltConfig> {
  for (const filename of CONFIG_FILES) {
    const configPath = resolve(projectRoot, filename);
    if (!existsSync(configPath)) {
      continue;
    }

    let config: VoltConfig;
    if (filename.endsWith('.ts')) {
      config = await loadWithJiti(configPath) ?? await loadWithDynamicImport(configPath);
    } else {
      config = await loadWithDynamicImport(configPath);
    }

    if (config) {
      return validateConfig(config, filename, options);
    }
  }

  if (options.strict) {
    const command = options.commandName ?? 'this command';
    throw new Error(`[volt] No config file found. ${command} requires one of: ${CONFIG_FILES.join(', ')}`);
  }
  console.warn('[volt] No config file found, using defaults.');
  return { ...DEFAULT_CONFIG };
}

async function loadWithJiti(configPath: string): Promise<VoltConfig | null> {
  const jitiModuleName = 'jiti';
  let jitiModule: unknown;
  try {
    jitiModule = await import(jitiModuleName);
  } catch (err) {
    if (isMissingOptionalJitiDependency(err)) {
      return null;
    }
    throw new Error(
      `[volt] Failed to initialize jiti while loading ${configPath}: ${err instanceof Error ? err.message : String(err)}`,
      { cause: err },
    );
  }

  const createJiti = (jitiModule as Record<string, unknown>).createJiti as
    ((path: string) => { import: (path: string) => Promise<unknown> }) | undefined;
  if (!createJiti) {
    return null;
  }

  const jiti = createJiti(configPath);
  try {
    const mod = await jiti.import(configPath);
    return (mod as Record<string, unknown>).default as VoltConfig ?? mod as VoltConfig;
  } catch (err) {
    throw new Error(
      `[volt] Failed to load TypeScript config at ${configPath} via jiti: ${err instanceof Error ? err.message : String(err)}`,
      { cause: err },
    );
  }
}

function isMissingOptionalJitiDependency(err: unknown): boolean {
  if (!(err instanceof Error)) {
    return false;
  }
  const message = err.message.toLowerCase();
  return message.includes("cannot find module 'jiti'")
    || message.includes('cannot find module "jiti"')
    || message.includes("cannot find package 'jiti'");
}

async function loadWithDynamicImport(configPath: string): Promise<VoltConfig> {
  const fileUrl = pathToFileURL(configPath).href;
  const mod = await import(fileUrl).catch((err) => {
    throw new Error(
      `[volt] Failed to load config at ${configPath}: ${err instanceof Error ? err.message : String(err)}`,
    );
  });
  return mod.default ?? mod;
}
