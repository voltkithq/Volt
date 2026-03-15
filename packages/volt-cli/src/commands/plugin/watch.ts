import { context as esbuildContext } from 'esbuild';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { resolvePluginBundlePath, resolvePluginProjectRoot, resolvePluginSourceEntry, loadPluginProject } from './project.js';

export async function watchPluginBuild(
  cwd: string,
  onRebuild: (ok: boolean) => Promise<void> | void,
): Promise<() => Promise<void>> {
  const projectRoot = resolvePluginProjectRoot(cwd);
  const project = loadPluginProject(projectRoot);
  const sourceEntry = resolvePluginSourceEntry(projectRoot);
  const outfile = resolvePluginBundlePath(projectRoot, project.manifest);
  const tsconfigPath = resolve(projectRoot, 'tsconfig.json');

  const ctx = await esbuildContext({
    entryPoints: [sourceEntry],
    outfile,
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    target: ['es2022'],
    sourcemap: false,
    minify: false,
    external: ['volt:*'],
    tsconfig: existsSync(tsconfigPath) ? tsconfigPath : undefined,
    logLevel: 'warning',
    plugins: [
      {
        name: 'volt-plugin-watch-rebuild',
        setup(build) {
          build.onEnd((result) => onRebuild(result.errors.length === 0));
        },
      },
    ],
  });

  await ctx.watch();
  return async () => {
    await ctx.dispose();
  };
}
