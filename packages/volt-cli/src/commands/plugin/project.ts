import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, extname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CONFIG_FILES } from '../../utils/config/constants.js';
import {
  type ManifestValidationResult,
  type PluginManifest,
  validatePluginManifest,
} from '../../utils/plugin-manifest.js';

export const PLUGIN_MANIFEST_FILE = 'volt-plugin.json';

const DEFAULT_PLUGIN_ENTRY_CANDIDATES = [
  'src/plugin.ts',
  'src/plugin.js',
  'plugin.ts',
  'plugin.js',
] as const;
const SUPPORTED_PLUGIN_ENTRY_EXTENSIONS = new Set(['.ts', '.js', '.mts', '.mjs', '.cts', '.cjs']);

interface CliPackageJson {
  version?: string;
}

export interface LoadedPluginProject {
  projectRoot: string;
  manifestPath: string;
  manifestResult: ManifestValidationResult;
  manifest: PluginManifest;
}

export function resolvePluginProjectRoot(startDir: string): string {
  return resolve(startDir);
}

export function resolvePluginManifestPath(projectRoot: string): string {
  return resolve(projectRoot, PLUGIN_MANIFEST_FILE);
}

export function loadPluginProject(projectRoot: string): LoadedPluginProject {
  const manifestPath = resolvePluginManifestPath(projectRoot);
  if (!existsSync(manifestPath)) {
    throw new Error(`[volt:plugin] Missing ${PLUGIN_MANIFEST_FILE} in ${projectRoot}`);
  }

  const raw = JSON.parse(readFileSync(manifestPath, 'utf8')) as unknown;
  const manifestResult = validatePluginManifest(raw);
  if (!manifestResult.valid || !manifestResult.manifest) {
    const summary = manifestResult.errors.map((error) => `${error.field}: ${error.message}`).join('; ');
    throw new Error(`[volt:plugin] Invalid plugin manifest: ${summary}`);
  }

  return {
    projectRoot,
    manifestPath,
    manifestResult,
    manifest: manifestResult.manifest,
  };
}

export function resolvePluginSourceEntry(projectRoot: string): string {
  for (const candidate of DEFAULT_PLUGIN_ENTRY_CANDIDATES) {
    const candidatePath = resolve(projectRoot, candidate);
    if (existsSync(candidatePath)) {
      ensureSupportedPluginEntryExtension(candidatePath);
      ensurePathWithinProject(projectRoot, candidatePath, 'plugin source entry');
      return candidatePath;
    }
  }

  throw new Error(
    `[volt:plugin] No plugin source entry found. Expected one of: ${DEFAULT_PLUGIN_ENTRY_CANDIDATES.join(', ')}`,
  );
}

export function resolvePluginBundlePath(projectRoot: string, manifest: PluginManifest): string {
  const bundlePath = resolve(projectRoot, manifest.backend);
  ensureSupportedPluginEntryExtension(bundlePath);
  ensurePathWithinProject(projectRoot, bundlePath, 'plugin bundle output');
  mkdirSync(dirname(bundlePath), { recursive: true });
  return bundlePath;
}

export function ensureSupportedPluginEntryExtension(entryPath: string): void {
  const extension = extname(entryPath).toLowerCase();
  if (SUPPORTED_PLUGIN_ENTRY_EXTENSIONS.has(extension)) {
    return;
  }
  throw new Error(
    `[volt:plugin] Unsupported plugin entry extension "${extension || '(none)'}". ` +
      `Expected one of: ${Array.from(SUPPORTED_PLUGIN_ENTRY_EXTENSIONS).join(', ')}`,
  );
}

export function ensurePathWithinProject(
  projectRoot: string,
  candidatePath: string,
  label: string,
): void {
  const relativePath = relative(resolve(projectRoot), resolve(candidatePath));
  if (relativePath === '' || (!relativePath.startsWith('..') && !isAbsolute(relativePath))) {
    return;
  }
  throw new Error(`[volt:plugin] ${label} must reside within project root: ${candidatePath}`);
}

export function findNearestVoltAppRoot(startDir: string): string | null {
  let current = resolve(startDir);
  while (true) {
    if (CONFIG_FILES.some((name) => existsSync(resolve(current, name)))) {
      return current;
    }
    const parent = dirname(current);
    if (parent === current) {
      return null;
    }
    current = parent;
  }
}

export function resolveVoltCliVersion(): string {
  const packageJsonPath = fileURLToPath(new URL('../../../package.json', import.meta.url));
  const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as CliPackageJson;
  return packageJson.version ?? '0.1.0';
}

export function toSafePackageName(name: string): string {
  return name.trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '-');
}

export const __testOnly = {
  DEFAULT_PLUGIN_ENTRY_CANDIDATES,
  SUPPORTED_PLUGIN_ENTRY_EXTENSIONS,
};
