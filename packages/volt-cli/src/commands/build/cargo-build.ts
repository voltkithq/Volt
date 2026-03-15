import { copyFileSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { extname, resolve } from 'node:path';

import type { VoltConfig } from 'voltkit';

import { convertPngToIco } from '../../utils/icon-converter.js';
import { toSafeBinaryName } from '../../utils/naming.js';
import { writeRuntimeArtifactManifest } from '../../utils/runtime-artifact.js';
import { artifactFileNameForTarget, inferBuildPlatform } from './platform.js';
import { readCargoMetadata, resolveRuntimeArtifact } from './runtime-artifact.js';
import { resolveBundledRunnerCrates } from './bundled-runner.js';

interface CargoBuildArgs {
  cwd: string;
  outputDir: string;
  target?: string;
  tempBundleDir: string;
  assetBundlePath: string;
  backendBundlePath: string;
  runnerConfigPath: string;
  config: VoltConfig;
}

export function buildWithCargo(args: CargoBuildArgs): void {
  const buildPlatform = inferBuildPlatform(args.target);
  const cargoMetadata = readCargoMetadata(args.cwd);
  const workspaceRoot = cargoMetadata?.workspace_root ?? resolveBundledWorkspaceRoot();
  const windowsIcon = resolveWindowsIcon(args, buildPlatform);

  execFileSync('cargo', buildCargoArgs(buildPlatform, args.target, args.config.devtools), {
    cwd: workspaceRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      VOLT_ASSET_BUNDLE: args.assetBundlePath,
      VOLT_BACKEND_BUNDLE: args.backendBundlePath,
      VOLT_RUNNER_CONFIG: args.runnerConfigPath,
      VOLT_APP_NAME: args.config.name,
      VOLT_APP_VERSION: args.config.version ?? '0.1.0',
      VOLT_UPDATE_PUBLIC_KEY: args.config.updater?.publicKey ?? '',
      ...(windowsIcon ? { VOLT_APP_ICON: windowsIcon } : {}),
    },
  });
  console.log('[volt] Native binary compiled successfully.');

  const targetRoot = cargoMetadata?.target_directory ?? resolve(workspaceRoot, 'target');
  const releaseDir = args.target
    ? resolve(targetRoot, args.target, 'release')
    : resolve(targetRoot, 'release');
  const { artifact, attemptedPaths } = resolveRuntimeArtifact(
    releaseDir,
    args.target,
    cargoMetadata,
  );
  if (!artifact) {
    const attempted =
      attemptedPaths.length > 0
        ? `\n[volt] Checked paths:\n  - ${attemptedPaths.join('\n  - ')}`
        : '\n[volt] No runtime artifact candidates could be derived from Cargo metadata.';
    throw new Error(`[volt] Failed to locate compiled runtime artifact.${attempted}`);
  }

  const outputFileName = `${toSafeBinaryName(args.config.name)}${extname(artifact.sourcePath)}`;
  const destBinary = resolve(args.outputDir, outputFileName);
  copyFileSync(artifact.sourcePath, destBinary);
  writeRuntimeArtifactManifest(args.outputDir, {
    schemaVersion: 1,
    artifactFileName: outputFileName,
    cargoArtifactKind: artifact.kind,
    cargoTargetName: artifact.targetName,
    rustTarget: args.target ?? null,
  });
  console.log(
    `[volt] Runtime artifact (${artifact.kind}:${artifact.targetName}) copied to ${destBinary}`,
  );

  if (buildPlatform === 'win32') {
    const helperFileName = artifactFileNameForTarget('volt-updater-helper', 'bin', buildPlatform);
    if (!helperFileName) {
      throw new Error('[volt] Build failed due to missing Windows updater helper artifact.');
    }
    const helperSourcePath = resolve(releaseDir, helperFileName);
    if (!existsSync(helperSourcePath)) {
      throw new Error('[volt] Build failed due to missing Windows updater helper artifact.');
    }
    const helperDestinationPath = resolve(args.outputDir, helperFileName);
    copyFileSync(helperSourcePath, helperDestinationPath);
    console.log(`[volt] Updater helper copied to ${helperDestinationPath}`);
  }
}

function resolveBundledWorkspaceRoot(): string {
  const bundledRunner = resolveBundledRunnerCrates();
  if (!bundledRunner) {
    throw new Error(
      '[volt] No Cargo workspace found and no bundled runner crates available.\n' +
        '  Make sure Rust is installed and either:\n' +
        '  - Run `volt build` inside a Cargo workspace containing volt-runner, or\n' +
        '  - Use a published @voltkit/volt-cli that includes bundled runner crates.',
    );
  }
  console.log('[volt] Using bundled runner crates (no local Cargo workspace found).');
  console.log(`[volt] Workspace root: ${bundledRunner}`);
  return bundledRunner;
}

function buildCargoArgs(
  buildPlatform: 'win32' | 'darwin' | 'linux',
  target: string | undefined,
  devtools: boolean | undefined,
): string[] {
  const cargoArgs = ['build', '--release', '-p', 'volt-runner'];
  if (buildPlatform === 'win32') {
    cargoArgs.push('-p', 'volt-updater-helper');
  }
  if (target) {
    cargoArgs.push('--target', target);
    console.log(`[volt] Cross-compilation target: ${target}`);
  }
  if (devtools) {
    cargoArgs.push('--features', 'devtools');
    console.log('[volt] Native devtools feature enabled for this build.');
  }
  return cargoArgs;
}

function resolveWindowsIcon(
  args: CargoBuildArgs,
  buildPlatform: 'win32' | 'darwin' | 'linux',
): string | undefined {
  if (buildPlatform !== 'win32') {
    return undefined;
  }
  const windowConfig = args.config.window as Record<string, unknown> | undefined;
  const iconField = windowConfig?.icon;
  if (typeof iconField !== 'string' || !iconField.trim()) {
    return undefined;
  }

  const resolvedIcon = resolve(args.cwd, iconField);
  if (!existsSync(resolvedIcon)) {
    return undefined;
  }
  if (resolvedIcon.endsWith('.ico')) {
    return resolvedIcon;
  }
  if (!resolvedIcon.endsWith('.png')) {
    return undefined;
  }

  try {
    const icoPath = convertPngToIco(resolvedIcon, args.tempBundleDir);
    console.log(`[volt] App icon converted for embedding: ${iconField}`);
    return icoPath;
  } catch (error) {
    console.warn(`[volt] Failed to convert icon to ICO format: ${error}`);
    return undefined;
  }
}
