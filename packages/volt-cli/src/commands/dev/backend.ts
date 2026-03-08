import { build as esbuild, type Plugin } from 'esbuild';
import { existsSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolveBackendEntry } from '../build/backend.js';
import { createScopedTempDirectory, recoverStaleScopedDirectories } from '../../utils/temp-artifacts.js';

const RUNTIME_MODULES_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  'runtime-modules',
);
const DEV_BACKEND_BUNDLE_PREFIX = 'bundle-';
const DEV_BACKEND_STALE_BUNDLE_MAX_AGE_MS = 12 * 60 * 60 * 1000;

const VOLT_RUNTIME_MODULE_TO_STEM: Record<string, string> = {
  'volt:ipc': 'volt-ipc',
  'volt:events': 'volt-events',
  'volt:window': 'volt-window',
  'volt:menu': 'volt-menu',
  'volt:globalShortcut': 'volt-global-shortcut',
  'volt:tray': 'volt-tray',
  'volt:db': 'volt-db',
  'volt:secureStorage': 'volt-secure-storage',
  'volt:clipboard': 'volt-clipboard',
  'volt:crypto': 'volt-crypto',
  'volt:os': 'volt-os',
  'volt:fs': 'volt-fs',
  'volt:dialog': 'volt-dialog',
  'volt:shell': 'volt-shell',
  'volt:notification': 'volt-notification',
  'volt:http': 'volt-http',
  'volt:updater': 'volt-updater',
};

export interface DevBackendLoadResult {
  loaded: boolean;
  backendEntryPath: string | null;
  backendBundlePath: string | null;
  dispose: () => void;
}

function resolveRuntimeModuleFile(stem: string): string {
  const jsPath = resolve(RUNTIME_MODULES_DIR, `${stem}.js`);
  if (existsSync(jsPath)) {
    return jsPath;
  }
  const tsPath = resolve(RUNTIME_MODULES_DIR, `${stem}.ts`);
  if (existsSync(tsPath)) {
    return tsPath;
  }
  throw new Error(`[volt] Missing dev runtime module shim: ${stem}`);
}

function buildRuntimeModulePathMap(): Record<string, string> {
  const entries = Object.entries(VOLT_RUNTIME_MODULE_TO_STEM)
    .map(([moduleSpecifier, stem]) => [moduleSpecifier, resolveRuntimeModuleFile(stem)] as const);
  return Object.fromEntries(entries);
}

function createRuntimeModuleAliasPlugin(runtimeModulePathMap: Record<string, string>): Plugin {
  return {
    name: 'volt-dev-runtime-module-aliases',
    setup(build) {
      build.onResolve({ filter: /^volt:/ }, (args) => {
        const mappedPath = runtimeModulePathMap[args.path];
        if (!mappedPath) {
          return {
            errors: [{ text: `[volt] Unsupported backend module in dev mode: ${args.path}` }],
          };
        }
        return { path: mappedPath };
      });
    },
  };
}

export async function loadBackendEntrypointForDev(
  projectRoot: string,
  configuredBackend: string | undefined,
): Promise<DevBackendLoadResult> {
  const backendEntryPath = resolveBackendEntry(projectRoot, configuredBackend);
  if (!backendEntryPath) {
    return {
      loaded: false,
      backendEntryPath: null,
      backendBundlePath: null,
      dispose: () => {},
    };
  }

  const runtimeModulePathMap = buildRuntimeModulePathMap();
  const backendTempRoot = resolve(projectRoot, '.volt-dev', 'dev-backend');
  const staleRecovery = recoverStaleScopedDirectories(backendTempRoot, {
    prefix: DEV_BACKEND_BUNDLE_PREFIX,
    staleAfterMs: DEV_BACKEND_STALE_BUNDLE_MAX_AGE_MS,
  });
  if (staleRecovery.removed > 0) {
    console.log(`[volt] Recovered ${staleRecovery.removed} stale dev backend bundle director${staleRecovery.removed === 1 ? 'y' : 'ies'}.`);
  }
  if (staleRecovery.failures > 0) {
    console.warn(`[volt] Failed to clean ${staleRecovery.failures} stale dev backend bundle director${staleRecovery.failures === 1 ? 'y' : 'ies'}.`);
  }

  const tempDir = createScopedTempDirectory(backendTempRoot, DEV_BACKEND_BUNDLE_PREFIX);
  const backendBundlePath = resolve(tempDir, 'backend.bundle.mjs');
  const tsconfigPath = resolve(projectRoot, 'tsconfig.json');

  const dispose = (): void => {
    rmSync(tempDir, { recursive: true, force: true });
  };

  try {
    await esbuild({
      entryPoints: [backendEntryPath],
      outfile: backendBundlePath,
      bundle: true,
      format: 'esm',
      platform: 'node',
      target: ['node22'],
      sourcemap: false,
      minify: false,
      external: ['voltkit', 'voltkit/*', '@voltkit/volt-native'],
      tsconfig: existsSync(tsconfigPath) ? tsconfigPath : undefined,
      logLevel: 'warning',
      plugins: [createRuntimeModuleAliasPlugin(runtimeModulePathMap)],
    });

    const backendUrl = `${pathToFileURL(backendBundlePath).href}?t=${Date.now()}`;
    await import(backendUrl);

    return {
      loaded: true,
      backendEntryPath,
      backendBundlePath,
      dispose,
    };
  } catch (error) {
    dispose();
    throw error;
  }
}

export const __testOnly = {
  VOLT_RUNTIME_MODULE_TO_STEM,
  buildRuntimeModulePathMap,
  createRuntimeModuleAliasPlugin,
  DEV_BACKEND_BUNDLE_PREFIX,
  DEV_BACKEND_STALE_BUNDLE_MAX_AGE_MS,
  recoverStaleScopedDirectories,
  createScopedTempDirectory,
};
