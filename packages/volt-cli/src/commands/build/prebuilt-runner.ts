import { copyFileSync, existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import type { VoltConfig } from 'voltkit';

import { convertPngToIco } from '../../utils/icon-converter.js';
import { toSafeBinaryName } from '../../utils/naming.js';
import { resolvePrebuiltRunner } from '../../utils/prebuilt-runner.js';
import { patchRunnerBinary } from '../../utils/binary-patcher.js';
import { patchExeIcon } from '../../utils/pe-icon-patcher.js';
import { writeRuntimeArtifactManifest } from '../../utils/runtime-artifact.js';

interface PrebuiltRunnerArgs {
  cwd: string;
  outputDir: string;
  buildPlatform: 'win32' | 'darwin' | 'linux';
  config: VoltConfig;
  tempBundleDir: string;
  assetBundlePath: string;
  backendBundlePath: string;
  runnerConfigPath: string;
}

export async function buildWithPrebuiltRunner(args: PrebuiltRunnerArgs): Promise<boolean> {
  const prebuiltRunner = await resolvePrebuiltRunner({
    platform: args.buildPlatform,
    arch: process.arch,
    cacheDir: resolve(args.cwd, '.volt-tmp', 'prebuilt-runners'),
    devtools: args.config.devtools,
  });
  if (!prebuiltRunner) {
    return false;
  }

  console.log('[volt] Using pre-built runner shell (no Cargo compilation required).');
  const binaryName = toSafeBinaryName(args.config.name);
  const outputFileName = `${binaryName}${args.buildPlatform === 'win32' ? '.exe' : ''}`;
  const destBinary = resolve(args.outputDir, outputFileName);

  try {
    patchRunnerBinary(prebuiltRunner, destBinary, {
      assetBundle: readFileSync(args.assetBundlePath),
      backendBundle: readFileSync(args.backendBundlePath),
      runnerConfig: readFileSync(args.runnerConfigPath),
    });
    console.log('[volt] Shell binary patched with app data.');
  } catch (error) {
    console.warn(`[volt] Binary patching failed, using sidecar files: ${error}`);
    copyFileSync(prebuiltRunner, destBinary);
    copyFileSync(args.assetBundlePath, resolve(args.outputDir, 'volt-assets.bin'));
    copyFileSync(args.backendBundlePath, resolve(args.outputDir, 'volt-backend.js'));
    copyFileSync(args.runnerConfigPath, resolve(args.outputDir, 'volt-config.json'));
  }

  writeRuntimeArtifactManifest(args.outputDir, {
    schemaVersion: 1,
    artifactFileName: outputFileName,
    cargoArtifactKind: 'bin',
    cargoTargetName: 'volt-runner',
    rustTarget: null,
  });
  console.log(`[volt] Runtime artifact copied to ${destBinary}`);

  if (args.buildPlatform === 'win32') {
    maybePatchWindowsIcon(args.config, args.cwd, args.tempBundleDir, destBinary);
  }

  return true;
}

function maybePatchWindowsIcon(
  config: VoltConfig,
  cwd: string,
  tempBundleDir: string,
  destBinary: string,
): void {
  const windowConfig = config.window as Record<string, unknown> | undefined;
  const iconField = windowConfig?.icon;
  if (typeof iconField !== 'string' || !iconField.trim()) {
    return;
  }

  const resolvedIcon = resolve(cwd, iconField);
  if (!existsSync(resolvedIcon)) {
    return;
  }

  let icoPath = resolvedIcon;
  if (resolvedIcon.endsWith('.png')) {
    try {
      icoPath = convertPngToIco(resolvedIcon, tempBundleDir);
    } catch {
      icoPath = resolvedIcon;
    }
  }

  if (patchExeIcon(destBinary, icoPath, config.name, config.version ?? '0.1.0')) {
    console.log(`[volt] App icon embedded into exe: ${iconField}`);
  }
}
