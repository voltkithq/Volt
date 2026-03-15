import { existsSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import type { VoltConfig } from 'voltkit';
import { pluginBuildCommand } from '../plugin/build.js';
import { loadPluginProject, resolvePluginManifestPath } from '../plugin/project.js';
import { watchPluginBuild } from '../plugin/watch.js';
import { createPluginHarnessDataRoot, startPluginHarness } from '../../utils/plugin-host-harness.js';

type PluginConfig = NonNullable<VoltConfig['plugins']>;

interface PluginDevelopmentLogger {
  log(message: string): void;
  warn(message: string): void;
  error(message: string): void;
}

interface PluginDevelopmentDeps {
  buildPlugin(cwd: string): Promise<void>;
  loadProject(cwd: string): ReturnType<typeof loadPluginProject>;
  startHarness: typeof startPluginHarness;
  createDataRoot(pluginId: string): string;
  watchPlugin: typeof watchPluginBuild;
  logger: PluginDevelopmentLogger;
}

const DEFAULT_DEPS: PluginDevelopmentDeps = {
  buildPlugin: (cwd) => pluginBuildCommand({ cwd }),
  loadProject: loadPluginProject,
  startHarness: startPluginHarness,
  createDataRoot: createPluginHarnessDataRoot,
  watchPlugin: watchPluginBuild,
  logger: console,
};

export async function startPluginDevelopment(
  appRoot: string,
  config: PluginConfig | undefined,
  deps: PluginDevelopmentDeps = DEFAULT_DEPS,
): Promise<() => Promise<void>> {
  const projectRoots = resolvePluginProjects(appRoot, config);
  const disposers = await Promise.all(projectRoots.map((projectRoot) => startWatchedPlugin(projectRoot, deps)));
  if (projectRoots.length > 0) {
    deps.logger.log(`[volt] Watching ${projectRoots.length} plugin${projectRoots.length === 1 ? '' : 's'} for rebuild/restart.`);
  }
  return async () => {
    for (const dispose of disposers.reverse()) {
      await dispose();
    }
  };
}

async function startWatchedPlugin(
  projectRoot: string,
  deps: PluginDevelopmentDeps,
): Promise<() => Promise<void>> {
  const project = deps.loadProject(projectRoot);
  let harness = await createAndActivateHarness(projectRoot, deps);
  let restartChain = Promise.resolve();

  const stopWatching = await deps.watchPlugin(projectRoot, async (ok) => {
    if (!ok) {
      deps.logger.error(`[volt] Plugin rebuild failed: ${project.manifest.id}`);
      return;
    }
    restartChain = restartChain.then(async () => {
      await harness.shutdown();
      harness = await createAndActivateHarness(projectRoot, deps);
      deps.logger.log(`[volt] Reloaded plugin ${project.manifest.id}`);
    });
    await restartChain;
  });

  return async () => {
    await stopWatching();
    await harness.shutdown();
  };
}

async function createAndActivateHarness(
  projectRoot: string,
  deps: PluginDevelopmentDeps,
) {
  await deps.buildPlugin(projectRoot);
  const project = deps.loadProject(projectRoot);
  const harness = await deps.startHarness({
    pluginId: project.manifest.id,
    backendEntry: resolve(projectRoot, project.manifest.backend),
    dataRoot: deps.createDataRoot(project.manifest.id),
    manifest: project.manifest,
  });
  await harness.activate();
  return harness;
}

function resolvePluginProjects(appRoot: string, config: PluginConfig | undefined): string[] {
  const pluginDirs = config?.pluginDirs ?? [];
  const enabled = new Set(config?.enabled ?? []);
  const projects = new Map<string, string>();

  for (const configuredDir of pluginDirs) {
    const baseDir = resolve(appRoot, configuredDir);
    const manifestPath = resolvePluginManifestPath(baseDir);
    if (existsPluginManifest(manifestPath)) {
      projects.set(baseDir, baseDir);
      continue;
    }

    for (const child of safeReadDirectory(baseDir)) {
      const childRoot = resolve(baseDir, child);
      if (existsPluginManifest(resolvePluginManifestPath(childRoot))) {
        projects.set(childRoot, childRoot);
      }
    }
  }

  const values = [...projects.values()];
  if (enabled.size === 0) {
    return values;
  }
  return values.filter((projectRoot) => enabled.has(loadPluginProject(projectRoot).manifest.id));
}

function existsPluginManifest(path: string): boolean {
  return existsSync(path);
}

function safeReadDirectory(path: string): string[] {
  try {
    return readdirSync(path, { withFileTypes: true })
      .filter((entry: { isDirectory(): boolean }) => entry.isDirectory())
      .map((entry: { name: string }) => entry.name);
  } catch {
    return [];
  }
}

export const __testOnly = {
  resolvePluginProjects,
};
