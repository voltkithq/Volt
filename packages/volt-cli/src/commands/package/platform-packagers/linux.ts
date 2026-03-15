import { chmodSync, copyFileSync, mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

import type { RuntimeArtifactDescriptor } from '../../../utils/runtime-artifact.js';
import {
  inferAppImageArchitecture,
  inferDebArchitecture,
  normalizeDebianControlVersion,
  runPackagingTool,
} from '../helpers.js';
import { generateAppRun, generateDesktopFile } from '../templates.js';
import type { PackageConfig } from '../types.js';

import { resolveLinuxPackageIcon, writeLinuxPackageIcon } from './assets.js';
import { copySidecarFiles } from './shared.js';

export async function packageLinux(
  appName: string,
  version: string,
  artifactVersion: string,
  binaryName: string,
  config: PackageConfig,
  outDir: string,
  runtimeArtifact: RuntimeArtifactDescriptor,
  format?: string,
  packageTarget?: string,
): Promise<string[]> {
  const formats = format ? [format] : ['appimage', 'deb'];
  const missingTools: string[] = [];
  const debControlVersion = normalizeDebianControlVersion(version);
  const linuxIcon = resolveLinuxPackageIcon(binaryName, config.icon);

  if (debControlVersion !== version) {
    console.warn(
      `[volt] Normalized Debian control version from "${version}" to "${debControlVersion}".`,
    );
  }
  if (linuxIcon.message) {
    console.log(`[volt] ${linuxIcon.message}`);
  }

  for (const fmt of formats) {
    if (fmt === 'appimage') {
      packageAppImage({
        appName,
        artifactVersion,
        binaryName,
        config,
        outDir,
        runtimeArtifact,
        packageTarget,
        linuxIcon,
        missingTools,
      });
    }

    if (fmt === 'deb') {
      packageDeb({
        appName,
        artifactVersion,
        binaryName,
        config,
        outDir,
        runtimeArtifact,
        packageTarget,
        linuxIcon,
        debControlVersion,
        missingTools,
      });
    }
  }

  return missingTools;
}

function packageAppImage(args: {
  appName: string;
  artifactVersion: string;
  binaryName: string;
  config: PackageConfig;
  outDir: string;
  runtimeArtifact: RuntimeArtifactDescriptor;
  packageTarget?: string;
  linuxIcon: ReturnType<typeof resolveLinuxPackageIcon>;
  missingTools: string[];
}): void {
  console.log('[volt] Creating AppImage...');
  const appImageArchitecture = inferAppImageArchitecture(
    args.packageTarget,
    args.runtimeArtifact.rustTarget,
  );
  const appDirPath = resolve(args.outDir, `${args.binaryName}.AppDir`);
  const usrBinDir = resolve(appDirPath, 'usr', 'bin');

  mkdirSync(usrBinDir, { recursive: true });
  const destBinary = resolve(usrBinDir, args.binaryName);
  copyFileSync(args.runtimeArtifact.absolutePath, destBinary);
  chmodSync(destBinary, 0o755);
  copySidecarFiles(dirname(args.runtimeArtifact.absolutePath), usrBinDir);

  writeFileSync(
    resolve(appDirPath, `${args.binaryName}.desktop`),
    generateDesktopFile(args.appName, args.binaryName, args.config, 'AppRun'),
  );
  writeLinuxPackageIcon(appDirPath, args.linuxIcon, { includeAppDirRoot: true });
  writeFileSync(resolve(appDirPath, 'AppRun'), generateAppRun(args.binaryName), { mode: 0o755 });

  const outputPath = resolve(
    args.outDir,
    `${args.binaryName}-${args.artifactVersion}-${appImageArchitecture}.AppImage`,
  );
  if (
    !runPackagingTool(
      'appimagetool',
      [appDirPath, outputPath],
      () => {
        console.log('[volt] appimagetool not found. Install it to create AppImages.');
        console.log(`[volt] AppDir structure created at: ${appDirPath}`);
      },
      '[volt] Failed to create AppImage package.',
    )
  ) {
    args.missingTools.push('appimagetool');
  }
  if (existsSync(outputPath)) {
    console.log(`[volt] AppImage created: ${outputPath}`);
  }
}

function packageDeb(args: {
  appName: string;
  artifactVersion: string;
  binaryName: string;
  config: PackageConfig;
  outDir: string;
  runtimeArtifact: RuntimeArtifactDescriptor;
  packageTarget?: string;
  linuxIcon: ReturnType<typeof resolveLinuxPackageIcon>;
  debControlVersion: string;
  missingTools: string[];
}): void {
  console.log('[volt] Creating .deb package...');
  const debArchitecture = inferDebArchitecture(args.packageTarget, args.runtimeArtifact.rustTarget);
  const debDir = resolve(
    args.outDir,
    `${args.binaryName}_${args.artifactVersion}_${debArchitecture}`,
  );
  const debBinDir = resolve(debDir, 'usr', 'bin');
  const debControlDir = resolve(debDir, 'DEBIAN');
  const debDesktopDir = resolve(debDir, 'usr', 'share', 'applications');

  mkdirSync(debBinDir, { recursive: true });
  mkdirSync(debControlDir, { recursive: true });
  mkdirSync(debDesktopDir, { recursive: true });

  const destBinary = resolve(debBinDir, args.binaryName);
  copyFileSync(args.runtimeArtifact.absolutePath, destBinary);
  chmodSync(destBinary, 0o755);
  copySidecarFiles(dirname(args.runtimeArtifact.absolutePath), debBinDir);

  writeFileSync(
    resolve(debControlDir, 'control'),
    [
      `Package: ${args.binaryName}`,
      `Version: ${args.debControlVersion}`,
      'Section: utils',
      'Priority: optional',
      `Architecture: ${debArchitecture}`,
      `Maintainer: ${args.appName} Developer`,
      `Description: ${args.appName}`,
      '  Desktop application built with Volt framework.',
      '',
    ].join('\n'),
  );
  writeFileSync(
    resolve(debDesktopDir, `${args.binaryName}.desktop`),
    generateDesktopFile(args.appName, args.binaryName, args.config),
  );
  writeLinuxPackageIcon(debDir, args.linuxIcon);

  const debPath = resolve(
    args.outDir,
    `${args.binaryName}_${args.artifactVersion}_${debArchitecture}.deb`,
  );
  if (
    !runPackagingTool(
      'dpkg-deb',
      ['--build', debDir, debPath],
      () => {
        console.log('[volt] dpkg-deb not found. Install dpkg to create .deb packages.');
      },
      '[volt] Failed to create deb package.',
    )
  ) {
    args.missingTools.push('dpkg-deb');
  }
  if (existsSync(debPath)) {
    console.log(`[volt] Deb package created: ${debPath}`);
  }
}
