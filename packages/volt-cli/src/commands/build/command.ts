import { build as viteBuild } from 'vite';
import { copyFileSync, existsSync, readFileSync } from 'node:fs';
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

import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runBuildPreflight, enforcePreflightResult } from '../../utils/preflight.js';
import { convertPngToIco } from '../../utils/icon-converter.js';
import { resolvePrebuiltRunner } from '../../utils/prebuilt-runner.js';
import { patchRunnerBinary } from '../../utils/binary-patcher.js';

export interface BuildOptions {
  target?: string;
}

/**
 * Locate the runner crate source bundled inside the volt-cli package.
 * Returns the path to the workspace root, or null if not found.
 */
function resolveBundledRunnerCrates(): string | null {
  const cliDistDir = dirname(fileURLToPath(import.meta.url));
  // runner-crates/ is a sibling of dist/ in the published package
  // From dist/commands/build/ we need to go up 3 levels to reach the package root
  const bundledPath = resolve(cliDistDir, '..', '..', '..', 'runner-crates');
  if (existsSync(resolve(bundledPath, 'Cargo.toml'))) {
    return bundledPath;
  }
  return null;
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

  prepareOutputDirectory(outputDir);
  const buildPlatform = inferBuildPlatform(options.target);
  const binaryName = toSafeBinaryName(config.name);

  if (!assetBundlePath || !backendBundlePath || !runnerConfigPath) {
    failBuild('[volt] Internal error: missing generated bundle paths.');
  }

  // ── Try pre-built runner (skips Cargo entirely) ─────────────────────
  const prebuiltCacheDir = resolve(cwd, '.volt-tmp', 'prebuilt-runners');
  const prebuiltRunner = await resolvePrebuiltRunner({
    platform: buildPlatform,
    arch: process.arch,
    cacheDir: prebuiltCacheDir,
  });

  if (prebuiltRunner && !config.devtools) {
    // Patch the pre-built shell binary with real app data — no Rust compilation needed.
    console.log('[volt] Using pre-built runner shell (no Cargo compilation required).');
    const ext = buildPlatform === 'win32' ? '.exe' : '';
    const outputFileName = `${binaryName}${ext}`;
    const destBinary = resolve(outputDir, outputFileName);

    try {
      patchRunnerBinary(prebuiltRunner, destBinary, {
        assetBundle: readFileSync(assetBundlePath!),
        backendBundle: readFileSync(backendBundlePath!),
        runnerConfig: readFileSync(runnerConfigPath!),
      });
      console.log('[volt] Shell binary patched with app data.');
    } catch (err) {
      // If patching fails (e.g. no sentinels found = Phase 1 binary), fall back to sidecar
      console.warn(`[volt] Binary patching failed, using sidecar files: ${err}`);
      copyFileSync(prebuiltRunner, destBinary);
      copyFileSync(assetBundlePath!, resolve(outputDir, 'volt-assets.bin'));
      copyFileSync(backendBundlePath!, resolve(outputDir, 'volt-backend.js'));
      copyFileSync(runnerConfigPath!, resolve(outputDir, 'volt-config.json'));
    }

    writeRuntimeArtifactManifest(outputDir, {
      schemaVersion: 1,
      artifactFileName: outputFileName,
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt-runner',
      rustTarget: options.target ?? null,
    });
    console.log(`[volt] Runtime artifact copied to ${destBinary}`);
  } else {
    // ── Cargo compilation path ──────────────────────────────────────────
    console.log('[volt] Compiling native binary...');
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
    let workspaceRoot: string;
    if (cargoMetadata?.workspace_root) {
      workspaceRoot = cargoMetadata.workspace_root;
    } else {
      const bundledRunner = resolveBundledRunnerCrates();
      if (!bundledRunner) {
        failBuild(
          '[volt] No Cargo workspace found and no bundled runner crates available.\n' +
          '  Make sure Rust is installed and either:\n' +
          '  - Run `volt build` inside a Cargo workspace containing volt-runner, or\n' +
          '  - Use a published @voltkit/volt-cli that includes bundled runner crates.',
        );
      }
      workspaceRoot = bundledRunner!;
      console.log('[volt] Using bundled runner crates (no local Cargo workspace found).');
    }
    console.log(`[volt] Workspace root: ${workspaceRoot}`);

    // Resolve and convert app icon for Windows resource embedding
    let appIconPath: string | undefined;
    if (buildPlatform === 'win32') {
      const windowConfig = config.window as Record<string, unknown> | undefined;
      const iconField = windowConfig?.icon;
      if (typeof iconField === 'string' && iconField.trim()) {
        const resolvedIcon = resolve(cwd, iconField);
        if (existsSync(resolvedIcon) && resolvedIcon.endsWith('.png')) {
          try {
            appIconPath = convertPngToIco(resolvedIcon, tempBundleDir!);
            console.log(`[volt] App icon converted for embedding: ${iconField}`);
          } catch (err) {
            console.warn(`[volt] Failed to convert icon to ICO format: ${err}`);
          }
        } else if (existsSync(resolvedIcon) && resolvedIcon.endsWith('.ico')) {
          appIconPath = resolvedIcon;
        }
      }
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
          ...(appIconPath ? { VOLT_APP_ICON: appIconPath } : {}),
        },
      });
      console.log('[volt] Native binary compiled successfully.');

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
  }

  cleanupGeneratedBundles();
  console.log(`[volt] Build complete. Output: ${outputDir}/`);
}
