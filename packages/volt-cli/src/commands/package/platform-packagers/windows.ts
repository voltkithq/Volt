import { copyFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { signWindows } from '../../../utils/signing.js';
import type { SigningArtifactResult } from '../../../utils/signing.js';
import type { RuntimeArtifactDescriptor } from '../../../utils/runtime-artifact.js';
import {
  normalizeMsixVersion,
  runPackagingTool,
  runPackagingToolWithFallback,
} from '../helpers.js';
import { generateMsixManifest, generateNsisScript } from '../templates.js';
import type { PackageConfig, PackageWindowsConfig, WindowsInstallMode } from '../types.js';

import { copySidecarFiles, SIDECAR_FILES } from './shared.js';
import { normalizeMsixIdentityName, normalizeMsixPublisher, writeMsixAssets } from './assets.js';

export async function packageWindows(
  appName: string,
  version: string,
  artifactVersion: string,
  binaryName: string,
  distDir: string,
  outDir: string,
  packageConfig: PackageConfig,
  runtimeArtifact: RuntimeArtifactDescriptor,
  format?: string,
  windowsConfig?: PackageWindowsConfig,
  installMode: WindowsInstallMode = 'perMachine',
  signing?: import('../../../utils/signing.js').ResolvedWindowsConfig,
  updaterHelperFileName?: string | null,
  signingResults: SigningArtifactResult[] = [],
): Promise<string[]> {
  const formats = format ? [format] : ['nsis'];
  const missingTools: string[] = [];

  if (signing) {
    signingResults.push(await signWindows(runtimeArtifact.absolutePath, signing));
    if (updaterHelperFileName) {
      const helperPath = resolve(distDir, updaterHelperFileName);
      if (existsSync(helperPath)) {
        signingResults.push(await signWindows(helperPath, signing));
      }
    }
  }

  for (const fmt of formats) {
    if (fmt === 'nsis') {
      await packageNsis({
        appName,
        artifactVersion,
        binaryName,
        distDir,
        outDir,
        runtimeArtifact,
        installMode,
        windowsConfig,
        signing,
        updaterHelperFileName,
        signingResults,
        missingTools,
      });
    }

    if (fmt === 'msix') {
      await packageMsix({
        appName,
        version,
        artifactVersion,
        binaryName,
        outDir,
        packageConfig,
        runtimeArtifact,
        windowsConfig,
        signing,
        updaterHelperFileName,
        distDir,
        signingResults,
        missingTools,
      });
    }
  }

  return missingTools;
}

async function packageNsis(args: {
  appName: string;
  artifactVersion: string;
  binaryName: string;
  distDir: string;
  outDir: string;
  runtimeArtifact: RuntimeArtifactDescriptor;
  installMode: WindowsInstallMode;
  windowsConfig?: PackageWindowsConfig;
  signing?: import('../../../utils/signing.js').ResolvedWindowsConfig;
  updaterHelperFileName?: string | null;
  signingResults: SigningArtifactResult[];
  missingTools: string[];
}): Promise<void> {
  console.log('[volt] Creating Windows NSIS installer...');

  const additionalFiles: string[] = [];
  if (args.updaterHelperFileName) additionalFiles.push(args.updaterHelperFileName);
  for (const sidecar of SIDECAR_FILES) {
    if (existsSync(resolve(args.distDir, sidecar))) {
      additionalFiles.push(sidecar);
    }
  }

  const scriptPath = resolve(args.outDir, 'installer.nsi');
  writeFileSync(
    scriptPath,
    generateNsisScript(
      args.appName,
      args.artifactVersion,
      args.binaryName,
      args.distDir,
      args.outDir,
      args.runtimeArtifact.fileName,
      additionalFiles,
      {
        installMode: args.installMode,
        silentAllUsers: args.windowsConfig?.silentAllUsers,
      },
    ),
  );

  if (
    !runPackagingTool(
      'makensis',
      [scriptPath],
      () => {
        console.log('[volt] NSIS not found. To create Windows installers, install NSIS.');
        console.log('[volt] The built binary is still available in dist-volt/.');
      },
      '[volt] Failed to create Windows NSIS installer.',
    )
  ) {
    args.missingTools.push('makensis');
  }

  const installerPath = resolve(
    args.outDir,
    `${args.binaryName}-${args.artifactVersion}-setup.exe`,
  );
  if (existsSync(installerPath)) {
    console.log(`[volt] NSIS installer created: ${installerPath}`);
    if (args.signing) {
      args.signingResults.push(await signWindows(installerPath, args.signing));
    }
  }
}

async function packageMsix(args: {
  appName: string;
  version: string;
  artifactVersion: string;
  binaryName: string;
  outDir: string;
  packageConfig: PackageConfig;
  runtimeArtifact: RuntimeArtifactDescriptor;
  windowsConfig?: PackageWindowsConfig;
  signing?: import('../../../utils/signing.js').ResolvedWindowsConfig;
  updaterHelperFileName?: string | null;
  distDir: string;
  signingResults: SigningArtifactResult[];
  missingTools: string[];
}): Promise<void> {
  console.log('[volt] Creating Windows MSIX package...');

  const stagingDir = resolve(args.outDir, `${args.binaryName}-msix-staging`);
  rmSync(stagingDir, { recursive: true, force: true });
  mkdirSync(stagingDir, { recursive: true });

  copyFileSync(
    args.runtimeArtifact.absolutePath,
    resolve(stagingDir, args.runtimeArtifact.fileName),
  );
  copySidecarFiles(args.distDir, stagingDir);

  if (args.updaterHelperFileName) {
    const helperPath = resolve(args.distDir, args.updaterHelperFileName);
    if (existsSync(helperPath)) {
      copyFileSync(helperPath, resolve(stagingDir, args.updaterHelperFileName));
    }
  }

  const msixAssets = writeMsixAssets(stagingDir, args.packageConfig.icon);
  writeFileSync(
    resolve(stagingDir, 'AppxManifest.xml'),
    generateMsixManifest({
      identityName: normalizeMsixIdentityName(
        args.windowsConfig?.msix?.identityName ?? args.packageConfig.identifier,
        args.binaryName,
      ),
      publisher: normalizeMsixPublisher(args.windowsConfig?.msix?.publisher),
      publisherDisplayName: args.windowsConfig?.msix?.publisherDisplayName ?? args.appName,
      displayName: args.windowsConfig?.msix?.displayName ?? args.appName,
      description: args.windowsConfig?.msix?.description ?? `${args.appName} desktop application`,
      executableFileName: args.runtimeArtifact.fileName,
      version: normalizeMsixVersion(args.version),
      square44Logo: msixAssets.square44Logo,
      square150Logo: msixAssets.square150Logo,
    }),
    'utf8',
  );

  const msixPath = resolve(args.outDir, `${args.binaryName}-${args.artifactVersion}.msix`);
  if (
    !runPackagingToolWithFallback(
      { command: 'makemsix', args: ['pack', '-d', stagingDir, '-p', msixPath] },
      { command: 'makeappx', args: ['pack', '/d', stagingDir, '/p', msixPath, '/o'] },
      () => {
        console.log(
          '[volt] makemsix/makeappx not found. Install Windows SDK packaging tools to build MSIX.',
        );
        console.log(`[volt] MSIX staging directory created: ${stagingDir}`);
      },
      '[volt] Failed to create Windows MSIX package.',
    )
  ) {
    args.missingTools.push('makemsix/makeappx');
  }

  if (existsSync(msixPath)) {
    console.log(`[volt] MSIX package created: ${msixPath}`);
    if (args.signing) {
      args.signingResults.push(await signWindows(msixPath, args.signing));
    }
  }
}
