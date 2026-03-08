import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import type {
  LoadedVoltTestConfig,
  LoadVoltTestConfigOptions,
  VoltTestConfig,
  VoltTestSuite,
} from './types.js';

const DEFAULT_TEST_CONFIG_FILES = [
  'volt.test.config.ts',
  'volt.test.config.mjs',
  'volt.test.config.js',
] as const;

export function defineTestConfig(config: VoltTestConfig): VoltTestConfig {
  return config;
}

export function validateTestConfig(config: VoltTestConfig, sourceLabel = 'config'): VoltTestConfig {
  if (!config || typeof config !== 'object') {
    throw new Error(`[volt:test] Invalid ${sourceLabel}: expected object.`);
  }

  if (!Array.isArray(config.suites) || config.suites.length === 0) {
    throw new Error(`[volt:test] Invalid ${sourceLabel}: expected at least one test suite.`);
  }

  const names = new Set<string>();
  for (const suite of config.suites) {
    validateSuite(suite, sourceLabel);
    if (names.has(suite.name)) {
      throw new Error(`[volt:test] Duplicate suite name "${suite.name}" in ${sourceLabel}.`);
    }
    names.add(suite.name);
  }

  if (config.timeoutMs !== undefined) {
    validateTimeoutMs(config.timeoutMs, `${sourceLabel}.timeoutMs`);
  }

  if (config.retries !== undefined) {
    validateRetryCount(config.retries, `${sourceLabel}.retries`);
  }

  if (config.artifactDir !== undefined) {
    if (typeof config.artifactDir !== 'string' || config.artifactDir.trim().length === 0) {
      throw new Error(`[volt:test] Invalid ${sourceLabel}.artifactDir: expected non-empty string.`);
    }
  }

  return config;
}

export async function loadVoltTestConfig(
  projectRoot: string,
  options: LoadVoltTestConfigOptions = {},
): Promise<LoadedVoltTestConfig> {
  const candidates = resolveConfigCandidates(projectRoot, options);
  for (const candidate of candidates) {
    if (!existsSync(candidate)) {
      continue;
    }

    const config = await loadConfigModule(candidate);
    return {
      configPath: candidate,
      config: validateTestConfig(config, candidate),
    };
  }

  if (options.strict ?? true) {
    const formattedCandidates = candidates
      .map((candidate) => candidate.replace(`${projectRoot}\\`, '').replace(`${projectRoot}/`, ''))
      .join(', ');
    throw new Error(
      `[volt:test] No test config found. Expected one of: ${formattedCandidates}.`,
    );
  }

  return {
    configPath: '',
    config: {
      suites: [],
    },
  };
}

function resolveConfigCandidates(projectRoot: string, options: LoadVoltTestConfigOptions): string[] {
  if (options.configPath) {
    return [resolve(projectRoot, options.configPath)];
  }
  return DEFAULT_TEST_CONFIG_FILES.map((filename) => resolve(projectRoot, filename));
}

async function loadConfigModule(configPath: string): Promise<VoltTestConfig> {
  if (configPath.endsWith('.ts')) {
    const viaJiti = await loadWithJiti(configPath);
    if (viaJiti) {
      return viaJiti;
    }
  }
  return loadWithDynamicImport(configPath);
}

async function loadWithJiti(configPath: string): Promise<VoltTestConfig | null> {
  const jitiModuleName = 'jiti';
  let jitiModule: unknown;
  try {
    jitiModule = await import(jitiModuleName);
  } catch (error) {
    if (isMissingOptionalJitiDependency(error)) {
      return null;
    }
    throw new Error(
      `[volt:test] Failed to initialize jiti for ${configPath}: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }

  const createJiti = (jitiModule as Record<string, unknown>).createJiti as
    | ((path: string) => { import: (path: string) => Promise<unknown> })
    | undefined;
  if (!createJiti) {
    return null;
  }

  const jiti = createJiti(configPath);
  const loaded = await jiti.import(configPath);
  return normalizeConfigModule(loaded, configPath);
}

function isMissingOptionalJitiDependency(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false;
  }
  const message = error.message.toLowerCase();
  return message.includes("cannot find module 'jiti'")
    || message.includes('cannot find module "jiti"')
    || message.includes("cannot find package 'jiti'");
}

async function loadWithDynamicImport(configPath: string): Promise<VoltTestConfig> {
  const fileUrl = pathToFileURL(configPath).href;
  const loaded = await import(fileUrl).catch((error: unknown) => {
    throw new Error(
      `[volt:test] Failed to load config at ${configPath}: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  });
  return normalizeConfigModule(loaded, configPath);
}

function normalizeConfigModule(moduleValue: unknown, sourceLabel: string): VoltTestConfig {
  const candidate = (moduleValue as Record<string, unknown>)?.default ?? moduleValue;
  if (!candidate || typeof candidate !== 'object') {
    throw new Error(`[volt:test] Invalid config export in ${sourceLabel}.`);
  }
  return candidate as VoltTestConfig;
}

function validateSuite(suite: VoltTestSuite, sourceLabel: string): void {
  if (!suite || typeof suite !== 'object') {
    throw new Error(`[volt:test] Invalid suite in ${sourceLabel}: expected object.`);
  }
  if (typeof suite.name !== 'string' || suite.name.trim().length === 0) {
    throw new Error(`[volt:test] Invalid suite in ${sourceLabel}: name must be a non-empty string.`);
  }
  if (typeof suite.run !== 'function') {
    throw new Error(`[volt:test] Invalid suite "${suite.name}" in ${sourceLabel}: missing run function.`);
  }
  if (suite.timeoutMs !== undefined) {
    validateTimeoutMs(suite.timeoutMs, `suite "${suite.name}".timeoutMs`);
  }
}

function validateTimeoutMs(timeoutMs: number, sourceLabel: string): void {
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error(`[volt:test] Invalid ${sourceLabel}: expected positive milliseconds.`);
  }
}

function validateRetryCount(retries: number, sourceLabel: string): void {
  if (!Number.isInteger(retries) || retries < 0) {
    throw new Error(`[volt:test] Invalid ${sourceLabel}: expected non-negative integer.`);
  }
}

export const __testOnly = {
  DEFAULT_TEST_CONFIG_FILES,
  normalizeConfigModule,
  resolveConfigCandidates,
  validateRetryCount,
  validateTimeoutMs,
};
