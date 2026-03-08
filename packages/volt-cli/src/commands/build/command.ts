import { build as viteBuild } from 'vite';
import { copyFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { extname, resolve } from 'node:path';
import { loadConfig } from '../../utils/config.js';
import { writeAssetBundle, validateBuildOutput } from '../../utils/bundler.js';
import { toSafeBinaryName } from '../../utils/naming.js';
import { writeRuntimeArtifactManifest } from '../../utils/runtime-artifact.js';
import { createScopedTempDirectory, recoverStaleScopedDirectories } from '../../utils/temp-artifacts.js';
import { buildBackendBundle, buildRunnerConfigPayload, resolveBackendEntry, writeRunnerConfig } from './backend.js';
import { cleanupAssetBundleIfExists, cleanupDirectoryIfExists, prepareOutputDirectory } from './fs-utils.js';
import { artifactFileNameForTarget, inferBuildPlatform } from './platform.js';
import { readCargoMetadata, resolveRuntimeArtifact } from './runtime-artifact.js';

export interface BuildOptions {
  target?: string;
}

/**
 * Build the application for production.
 * 1. Load volt.config.ts
 * 2. Run Vite production build
 * 3. Bundle frontend assets into a binary format
 * 4. Bundle backend code for Boa
 * 5. Compile volt-runner with embedded frontend + backend bundles
 * 6. Output single binary to dist-volt/
 */
export async function buildCommand(options: BuildOptions): Promise<void> {
  const cwd = process.cwd();
  const buildTempRoot = resolve(cwd, '.volt-tmp', 'build');
  const staleRecovery = recoverStaleScopedDirectories(buildTempRoot, {
    prefix: 'run-',
    staleAfterMs: 6 * 60 * 60 * 1000,
  });

  if (staleRecovery.removed > 0) {
    console.log(`[volt] Recovered ${staleRecovery.removed} stale temporary build director${staleRecovery.removed === 1 ? 'y' : 'ies'}.`);
  }
  if (staleRecovery.failures > 0) {
    console.warn(`[volt] Failed to clean ${staleRecovery.failures} stale temporary build director${staleRecovery.failures === 1 ? 'y' : 'ies'}.`);
  }

  const staleAssetBundlePath = resolve(cwd, '.volt-assets.bin');
  if (cleanupAssetBundleIfExists(staleAssetBundlePath)) {
    console.log(`[volt] Removed stale asset bundle from previous run: ${staleAssetBundlePath}`);
  }

  console.log('[volt] Building for production...');

  const config = await loadConfig(cwd, { strict: true, commandName: 'build' });
  console.log(`[volt] App: ${config.name}`);

  const outDir = config.build?.outDir ?? 'dist';
  const outputDir = resolve(cwd, 'dist-volt');
  let assetBundlePath: string | null = null;
  let backendBundlePath: string | null = null;
  let runnerConfigPath: string | null = null;
  let tempBundleDir: string | null = null;

  const cleanupGeneratedBundles = (): void => {
    try {
      const removedAsset = cleanupAssetBundleIfExists(assetBundlePath);
      if (removedAsset) {
        console.log(`[volt] Removed stale asset bundle: ${assetBundlePath}`);
      }
      const removedBackend = cleanupAssetBundleIfExists(backendBundlePath);
      if (removedBackend) {
        console.log(`[volt] Removed stale backend bundle: ${backendBundlePath}`);
      }
      const removedRunnerConfig = cleanupAssetBundleIfExists(runnerConfigPath);
      if (removedRunnerConfig) {
        console.log(`[volt] Removed stale runner config: ${runnerConfigPath}`);
      }
      const removedTempDir = cleanupDirectoryIfExists(tempBundleDir);
      if (removedTempDir) {
        console.log(`[volt] Removed temporary build directory: ${tempBundleDir}`);
      }
    } catch (cleanupError) {
      console.warn('[volt] Failed to remove generated build artifacts:', cleanupError);
    }
  };

  const failBuild = (message: string, err?: unknown): never => {
    cleanupGeneratedBundles();
    if (err === undefined) {
      console.error(message);
    } else {
      console.error(message, err);
    }
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
  } catch (err) {
    failBuild('[volt] Vite build failed:', err);
  }

  if (!validateBuildOutput(cwd, outDir)) {
    failBuild(`[volt] Build output validation failed: missing index.html in ${outDir}/`);
  }

  console.log('[volt] Bundling frontend assets...');
  assetBundlePath = writeAssetBundle(cwd, outDir, '.volt-assets.bin');
  console.log(`[volt] Asset bundle created: ${assetBundlePath}`);

  tempBundleDir = createScopedTempDirectory(buildTempRoot, 'run-');
  backendBundlePath = resolve(tempBundleDir, 'backend.bundle.mjs');
  runnerConfigPath = resolve(tempBundleDir, 'runner.config.json');
  const configuredBackend = config.backend;
  let backendEntryPath: string | null = null;
  const runnerConfigPayload = buildRunnerConfigPayload(config);
  try {
    backendEntryPath = resolveBackendEntry(cwd, configuredBackend);
  } catch (err) {
    failBuild('[volt] Failed to resolve backend entry:', err);
  }

  try {
    await buildBackendBundle(cwd, backendEntryPath, backendBundlePath);
    if (backendEntryPath) {
      console.log(`[volt] Backend bundled from ${backendEntryPath}`);
    } else {
      console.log('[volt] No backend entry found, using empty backend bundle.');
    }
  } catch (err) {
    failBuild('[volt] Backend bundle failed:', err);
  }

  try {
    writeRunnerConfig(runnerConfigPath, runnerConfigPayload);
    console.log(`[volt] Runner config prepared: ${runnerConfigPath}`);
  } catch (err) {
    failBuild('[volt] Failed to generate runner config:', err);
  }

  console.log('[volt] Compiling native binary...');
  prepareOutputDirectory(outputDir);

  const buildPlatform = inferBuildPlatform(options.target);
  const cargoPackages = ['volt-runner'];
  if (buildPlatform === 'win32') {
    cargoPackages.push('volt-updater-helper');
  }
  const cargoArgs = ['build', '--release'];
  for (const pkg of cargoPackages) {
    cargoArgs.push('-p', pkg);
  }
  if (options.target) {
    cargoArgs.push('--target', options.target);
    console.log(`[volt] Cross-compilation target: ${options.target}`);
  }
  if (config.devtools) {
    cargoArgs.push('--features', 'devtools');
    console.log('[volt] Native devtools feature enabled for this build.');
  }

  const cargoMetadata = readCargoMetadata(cwd);
  const workspaceRoot = cargoMetadata?.workspace_root ?? cwd;
  console.log(`[volt] Workspace root: ${workspaceRoot}`);

  if (!assetBundlePath || !backendBundlePath || !runnerConfigPath) {
    failBuild('[volt] Internal error: missing generated bundle paths.');
  }

  try {
    execFileSync('cargo', cargoArgs, {
      cwd: workspaceRoot,
      stdio: 'inherit',
      env: {
        ...process.env,
        VOLT_ASSET_BUNDLE: assetBundlePath,
        VOLT_BACKEND_BUNDLE: backendBundlePath,
        VOLT_RUNNER_CONFIG: runnerConfigPath,
        VOLT_APP_NAME: config.name,
        VOLT_APP_VERSION: config.version ?? '0.1.0',
        VOLT_UPDATE_PUBLIC_KEY: config.updater?.publicKey ?? '',
      },
    });
    console.log('[volt] Native binary compiled successfully.');

    const binaryName = toSafeBinaryName(config.name);
    const targetRoot = cargoMetadata?.target_directory ?? resolve(workspaceRoot, 'target');
    const releaseDir = options.target
      ? resolve(targetRoot, options.target, 'release')
      : resolve(targetRoot, 'release');
    const { artifact, attemptedPaths } = resolveRuntimeArtifact(
      releaseDir,
      options.target,
      cargoMetadata,
    );
    if (!artifact) {
      console.error('[volt] Failed to locate compiled runtime artifact.');
      if (attemptedPaths.length > 0) {
        console.error(`[volt] Checked paths:\n  - ${attemptedPaths.join('\n  - ')}`);
      } else {
        console.error('[volt] No runtime artifact candidates could be derived from Cargo metadata.');
      }
    }
    const runtimeArtifact = artifact ?? failBuild('[volt] Build failed due to missing runtime artifact.');

    const runtimeArtifactExt = extname(runtimeArtifact.sourcePath);
    const outputFileName = `${binaryName}${runtimeArtifactExt}`;
    const destBinary = resolve(outputDir, outputFileName);
    copyFileSync(runtimeArtifact.sourcePath, destBinary);
    writeRuntimeArtifactManifest(outputDir, {
      schemaVersion: 1,
      artifactFileName: outputFileName,
      cargoArtifactKind: runtimeArtifact.kind,
      cargoTargetName: runtimeArtifact.targetName,
      rustTarget: options.target ?? null,
    });
    console.log(
      `[volt] Runtime artifact (${runtimeArtifact.kind}:${runtimeArtifact.targetName}) copied to ${destBinary}`,
    );

    if (buildPlatform === 'win32') {
      const helperFileName =
        artifactFileNameForTarget('volt-updater-helper', 'bin', buildPlatform)
        ?? failBuild('[volt] Build failed due to missing Windows updater helper artifact.');
      const helperSourcePath = resolve(releaseDir, helperFileName);
      if (!existsSync(helperSourcePath)) {
        failBuild('[volt] Build failed due to missing Windows updater helper artifact.');
      }
      const helperDestinationPath = resolve(outputDir, helperFileName);
      copyFileSync(helperSourcePath, helperDestinationPath);
      console.log(`[volt] Updater helper copied to ${helperDestinationPath}`);
    }
  } catch (err) {
    failBuild('[volt] Cargo build failed:', err);
  }

  cleanupGeneratedBundles();
  console.log(`[volt] Build complete. Output: ${outputDir}/`);
}
