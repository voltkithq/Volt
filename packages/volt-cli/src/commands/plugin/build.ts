import { buildBackendBundle } from '../build/backend.js';
import { loadPluginProject, resolvePluginBundlePath, resolvePluginProjectRoot, resolvePluginSourceEntry } from './project.js';

export interface PluginBuildOptions {
  cwd?: string;
}

export async function pluginBuildCommand(options: PluginBuildOptions = {}): Promise<void> {
  const projectRoot = resolvePluginProjectRoot(options.cwd ?? process.cwd());
  const project = loadPluginProject(projectRoot);
  const sourceEntry = resolvePluginSourceEntry(projectRoot);
  const bundlePath = resolvePluginBundlePath(projectRoot, project.manifest);
  await buildBackendBundle(projectRoot, sourceEntry, bundlePath);
  console.log(`[volt:plugin] Built ${project.manifest.id} -> ${bundlePath}`);
}
