import { build as viteBuild } from 'vite';
import { resolve } from 'node:path';

import { loadConfig } from '../../utils/config.js';
import { writeAssetBundle, validateBuildOutput } from '../../utils/bundler.js';
import {
  createScopedTempDirectory,
  recoverStaleScopedDirectories,
} from '../../utils/temp-artifacts.js';
import { runBuildPreflight, enforcePreflightResult } from '../../utils/preflight.js';
import {
  buildBackendBundle,
  buildRunnerConfigPayload,
  resolveBackendEntry,
  writeRunnerConfig,
} from './backend.js';
import {
  cleanupAssetBundleIfExists,
  cleanupDirectoryIfExists,
  prepareOutputDirectory,
} from './fs-utils.js';
import { inferBuildPlatform } from './platform.js';
import { readCargoMetadata } from './runtime-artifact.js';
import { buildWithCargo } from './cargo-build.js';
import { resolveBundledRunnerCrates } from './bundled-runner.js';
import { buildWithPrebuiltRunner } from './prebuilt-runner.js';

export interface BuildOptions {
  target?: string;
}

export async function buildCommand(options: BuildOptions): Promise<void> {
  const cwd = process.cwd();
  const buildTempRoot = resolve(cwd, '.volt-tmp', 'build');
  const staleRecovery = recoverStaleScopedDirectories(buildTempRoot, {
    prefix: 'run-',
    staleAfterMs: 6 * 60 * 60 * 1000,
  });

  if (staleRecovery.removed > 0) {
    console.log(
      `[volt] Recovered ${staleRecovery.removed} stale temporary build director${staleRecovery.removed === 1 ? 'y' : 'ies'}.`,
    );
  }
  if (staleRecovery.failures > 0) {
    console.warn(
      `[volt] Failed to clean ${staleRecovery.failures} stale temporary build director${staleRecovery.failures === 1 ? 'y' : 'ies'}.`,
    );
  }

  const staleAssetBundlePath = resolve(cwd, '.volt-assets.bin');
  if (cleanupAssetBundleIfExists(staleAssetBundlePath)) {
    console.log(`[volt] Removed stale asset bundle from previous run: ${staleAssetBundlePath}`);
  }

  console.log('[volt] Building for production...');
  const config = await loadConfig(cwd, { strict: true, commandName: 'build' });
  console.log(`[volt] App: ${config.name}`);

  const hasCargoWorkspace = Boolean(readCargoMetadata(cwd)?.workspace_root);
  const hasBundledRunner = Boolean(resolveBundledRunnerCrates());
  enforcePreflightResult(
    runBuildPreflight(cwd, config, {
      hasPrebuiltRunner: hasCargoWorkspace || hasBundledRunner,
      target: options.target,
    }),
  );

  const outDir = config.build?.outDir ?? 'dist';
  const outputDir = resolve(cwd, 'dist-volt');
  let assetBundlePath: string | null = null;
  let backendBundlePath: string | null = null;
  let runnerConfigPath: string | null = null;
  let tempBundleDir: string | null = null;

  const cleanupGeneratedBundles = (): void => {
    try {
      if (cleanupAssetBundleIfExists(assetBundlePath)) {
        console.log(`[volt] Removed stale asset bundle: ${assetBundlePath}`);
      }
      if (cleanupAssetBundleIfExists(backendBundlePath)) {
        console.log(`[volt] Removed stale backend bundle: ${backendBundlePath}`);
      }
      if (cleanupAssetBundleIfExists(runnerConfigPath)) {
        console.log(`[volt] Removed stale runner config: ${runnerConfigPath}`);
      }
      if (cleanupDirectoryIfExists(tempBundleDir)) {
        console.log(`[volt] Removed temporary build directory: ${tempBundleDir}`);
      }
    } catch (cleanupError) {
      console.warn('[volt] Failed to remove generated build artifacts:', cleanupError);
    }
  };

  const failBuild = (message: string, err?: unknown): never => {
    cleanupGeneratedBundles();
    console.error(message, err ?? '');
    process.exit(1);
  };

  try {
    await viteBuild({
      root: cwd,
      build: {
        outDir: resolve(cwd, outDir),
        emptyOutDir: true,
      },
      logLevel: 'info',
    });
    console.log(`[volt] Frontend assets built to ${outDir}/`);
  } catch (error) {
    failBuild('[volt] Vite build failed:', error);
  }

  if (!validateBuildOutput(cwd, outDir)) {
    failBuild(`[volt] Build output validation failed: missing index.html in ${outDir}/`);
  }

  try {
    assetBundlePath = writeAssetBundle(cwd, outDir, '.volt-assets.bin');
    console.log(`[volt] Asset bundle created: ${assetBundlePath}`);

    tempBundleDir = createScopedTempDirectory(buildTempRoot, 'run-');
    backendBundlePath = resolve(tempBundleDir, 'backend.bundle.mjs');
    runnerConfigPath = resolve(tempBundleDir, 'runner.config.json');

    const backendEntryPath = resolveBackendEntry(cwd, config.backend);
    await buildBackendBundle(cwd, backendEntryPath, backendBundlePath);
    console.log(
      backendEntryPath
        ? `[volt] Backend bundled from ${backendEntryPath}`
        : '[volt] No backend entry found, using empty backend bundle.',
    );

    writeRunnerConfig(runnerConfigPath, buildRunnerConfigPayload(config));
    console.log(`[volt] Runner config prepared: ${runnerConfigPath}`);
  } catch (error) {
    failBuild('[volt] Failed to prepare generated build artifacts:', error);
  }

  if (!assetBundlePath || !backendBundlePath || !runnerConfigPath || !tempBundleDir) {
    failBuild('[volt] Internal error: missing generated bundle paths.');
  }
  const generatedPaths = {
    assetBundlePath: assetBundlePath!,
    backendBundlePath: backendBundlePath!,
    runnerConfigPath: runnerConfigPath!,
    tempBundleDir: tempBundleDir!,
  };

  prepareOutputDirectory(outputDir);
  const buildPlatform = inferBuildPlatform(options.target);

  try {
    const usedPrebuiltRunner = await buildWithPrebuiltRunner({
      cwd,
      outputDir,
      buildPlatform,
      config,
      tempBundleDir: generatedPaths.tempBundleDir,
      assetBundlePath: generatedPaths.assetBundlePath,
      backendBundlePath: generatedPaths.backendBundlePath,
      runnerConfigPath: generatedPaths.runnerConfigPath,
    });

    if (!usedPrebuiltRunner) {
      buildWithCargo({
        cwd,
        outputDir,
        target: options.target,
        tempBundleDir: generatedPaths.tempBundleDir,
        assetBundlePath: generatedPaths.assetBundlePath,
        backendBundlePath: generatedPaths.backendBundlePath,
        runnerConfigPath: generatedPaths.runnerConfigPath,
        config,
      });
    }
  } catch (error) {
    failBuild('[volt] Build failed:', error);
  }

  cleanupGeneratedBundles();
  console.log(`[volt] Build complete. Output: ${outputDir}/`);
}
