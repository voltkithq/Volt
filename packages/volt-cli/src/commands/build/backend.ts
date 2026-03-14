import { build as esbuild } from 'esbuild';
import { extname, isAbsolute, relative, resolve } from 'node:path';
import { existsSync, realpathSync, writeFileSync } from 'node:fs';

const DEFAULT_BACKEND_ENTRY_CANDIDATES = [
  'src/backend.ts',
  'src/backend.js',
  'backend.ts',
  'backend.js',
] as const;

const SUPPORTED_BACKEND_ENTRY_EXTENSIONS = new Set(['.ts', '.js', '.mts', '.mjs', '.cts', '.cjs']);

export function ensureSupportedBackendExtension(entryPath: string): void {
  const extension = extname(entryPath).toLowerCase();
  if (SUPPORTED_BACKEND_ENTRY_EXTENSIONS.has(extension)) {
    return;
  }
  throw new Error(
    `[volt] Unsupported backend entry extension "${extension || '(none)'}". `
      + `Expected one of: ${Array.from(SUPPORTED_BACKEND_ENTRY_EXTENSIONS).join(', ')}`,
  );
}

export function ensureBackendEntryWithinProject(projectRoot: string, entryPath: string): void {
  const rootRealPath = realpathSync(projectRoot);
  const entryRealPath = realpathSync(entryPath);
  const relativePath = relative(rootRealPath, entryRealPath);
  if (
    relativePath === ''
    || (!relativePath.startsWith('..') && !isAbsolute(relativePath))
  ) {
    return;
  }
  throw new Error(
    `[volt] Configured backend entry must reside within project root: ${entryPath}`,
  );
}

export function buildRunnerConfigPayload(config: {
  name: string;
  devtools?: boolean;
  permissions?: string[];
  window?: unknown;
  runtime?: unknown;
  updater?: unknown;
  plugins?: unknown;
}): Record<string, unknown> {
  const payload: Record<string, unknown> = {
    name: config.name,
  };

  if (typeof config.devtools === 'boolean') {
    payload['devtools'] = config.devtools;
  }
  if (Array.isArray(config.permissions)) {
    payload['permissions'] = [...config.permissions];
  }
  if (config.window && typeof config.window === 'object') {
    payload['window'] = config.window;
  }
  if (config.runtime && typeof config.runtime === 'object') {
    payload['runtime'] = config.runtime;
  }
  if (config.updater && typeof config.updater === 'object') {
    payload['updater'] = config.updater;
  }
  if (config.plugins && typeof config.plugins === 'object') {
    payload['plugins'] = config.plugins;
  }

  const rawConfig = config as unknown as Record<string, unknown>;
  const webview = rawConfig['webview'];
  if (webview && typeof webview === 'object') {
    payload['webview'] = webview;
  }

  const fsBaseDir = rawConfig['fsBaseDir'];
  if (typeof fsBaseDir === 'string' && fsBaseDir.trim().length > 0) {
    payload['fsBaseDir'] = fsBaseDir;
  }

  const baseDir = rawConfig['baseDir'];
  if (typeof baseDir === 'string' && baseDir.trim().length > 0) {
    payload['baseDir'] = baseDir;
  }

  const runtimePoolSize = rawConfig['runtimePoolSize'];
  if (typeof runtimePoolSize === 'number' && Number.isInteger(runtimePoolSize) && runtimePoolSize > 0) {
    payload['runtimePoolSize'] = runtimePoolSize;
  }

  return payload;
}

export function writeRunnerConfig(path: string, payload: Record<string, unknown>): void {
  writeFileSync(path, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

export function resolveBackendEntry(
  projectRoot: string,
  configuredBackend: string | undefined,
): string | null {
  if (configuredBackend && configuredBackend.trim().length > 0) {
    const explicitPath = resolve(projectRoot, configuredBackend);
    if (!existsSync(explicitPath)) {
      throw new Error(`[volt] Configured backend entry does not exist: ${configuredBackend}`);
    }
    ensureSupportedBackendExtension(explicitPath);
    ensureBackendEntryWithinProject(projectRoot, explicitPath);
    return explicitPath;
  }

  for (const candidate of DEFAULT_BACKEND_ENTRY_CANDIDATES) {
    const candidatePath = resolve(projectRoot, candidate);
    if (existsSync(candidatePath)) {
      return candidatePath;
    }
  }

  return null;
}

export async function buildBackendBundle(
  projectRoot: string,
  backendEntryPath: string | null,
  backendBundlePath: string,
): Promise<void> {
  if (!backendEntryPath) {
    writeFileSync(backendBundlePath, 'void 0;\n', 'utf8');
    return;
  }

  const tsconfigPath = resolve(projectRoot, 'tsconfig.json');
  await esbuild({
    entryPoints: [backendEntryPath],
    outfile: backendBundlePath,
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    target: ['es2022'],
    sourcemap: false,
    minify: false,
    external: ['volt:*'],
    tsconfig: existsSync(tsconfigPath) ? tsconfigPath : undefined,
    logLevel: 'warning',
  });
}
