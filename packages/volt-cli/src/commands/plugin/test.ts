import { createPluginHarnessDataRoot, startPluginHarness } from '../../utils/plugin-host-harness.js';
import { pluginBuildCommand } from './build.js';
import { loadPluginProject, resolvePluginBundlePath, resolvePluginProjectRoot } from './project.js';

interface PluginTestDeps {
  buildPlugin(cwd: string): Promise<void>;
  createDataRoot(pluginId: string): string;
  startHarness: typeof startPluginHarness;
}

export interface PluginTestOptions {
  cwd?: string;
}

const DEFAULT_DEPS: PluginTestDeps = {
  buildPlugin: (cwd) => pluginBuildCommand({ cwd }),
  createDataRoot: createPluginHarnessDataRoot,
  startHarness: startPluginHarness,
};

export async function pluginTestCommand(
  deps: PluginTestDeps = DEFAULT_DEPS,
  options: PluginTestOptions = {},
): Promise<void> {
  const projectRoot = resolvePluginProjectRoot(options.cwd ?? process.cwd());
  await deps.buildPlugin(projectRoot);
  const project = loadPluginProject(projectRoot);
  const harness = await deps.startHarness({
    pluginId: project.manifest.id,
    backendEntry: resolvePluginBundlePath(projectRoot, project.manifest),
    dataRoot: deps.createDataRoot(project.manifest.id),
    manifest: project.manifest,
  });

  try {
    await harness.activate();
    console.log(`[volt:plugin:test] PASS activate ${project.manifest.id}`);
    for (const command of project.manifest.contributes?.commands ?? []) {
      const result = await harness.invokeCommand(command.id, null);
      console.log(
        `[volt:plugin:test] PASS command ${command.id} -> ${JSON.stringify(result ?? null)}`,
      );
    }
  } catch (error) {
    console.error(
      `[volt:plugin:test] FAIL ${error instanceof Error ? error.message : String(error)}`,
    );
    process.exitCode = 1;
  } finally {
    await harness.shutdown();
  }
}

export const __testOnly = {
  DEFAULT_DEPS,
};
