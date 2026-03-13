import { resolve, extname } from 'node:path';
import { chmodSync, copyFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { signMacOS, signWindows } from '../../utils/signing.js';
import type { SigningArtifactResult } from '../../utils/signing.js';
import type { RuntimeArtifactDescriptor } from '../../utils/runtime-artifact.js';
import {
  inferAppImageArchitecture,
  inferDebArchitecture,
  normalizeDebianControlVersion,
  normalizeMsixVersion,
  runPackagingTool,
  runPackagingToolWithFallback,
} from './helpers.js';
import {
  generateAppRun,
  generateDesktopFile,
  generateInfoPlist,
  generateMsixManifest,
  generateNsisScript,
} from './templates.js';
import type { PackageConfig, PackageWindowsConfig, WindowsInstallMode } from './types.js';

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
  signing?: import('../../utils/signing.js').ResolvedWindowsConfig,
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
      console.log('[volt] Creating Windows NSIS installer...');

      const additionalFiles: string[] = [];
      if (updaterHelperFileName) additionalFiles.push(updaterHelperFileName);
      // Include sidecar files if present (pre-built runner mode)
      for (const sidecar of ['volt-assets.bin', 'volt-backend.js', 'volt-config.json']) {
        if (existsSync(resolve(distDir, sidecar))) {
          additionalFiles.push(sidecar);
        }
      }

      const nsisScript = generateNsisScript(
        appName,
        artifactVersion,
        binaryName,
        distDir,
        outDir,
        runtimeArtifact.fileName,
        additionalFiles,
        {
          installMode,
          silentAllUsers: windowsConfig?.silentAllUsers,
        },
      );
      const scriptPath = resolve(outDir, 'installer.nsi');
      writeFileSync(scriptPath, nsisScript);

      if (!runPackagingTool(
        'makensis',
        [scriptPath],
        () => {
          console.log('[volt] NSIS not found. To create Windows installers, install NSIS.');
          console.log('[volt] The built binary is still available in dist-volt/.');
        },
        '[volt] Failed to create Windows NSIS installer.',
      )) {
        missingTools.push('makensis');
      }

      const installerPath = resolve(outDir, `${binaryName}-${artifactVersion}-setup.exe`);
      if (existsSync(installerPath)) {
        console.log(`[volt] NSIS installer created: ${installerPath}`);
      }

      if (signing && existsSync(installerPath)) {
        signingResults.push(await signWindows(installerPath, signing));
      }
    }

    if (fmt === 'msix') {
      console.log('[volt] Creating Windows MSIX package...');

      const stagingDir = resolve(outDir, `${binaryName}-msix-staging`);
      rmSync(stagingDir, { recursive: true, force: true });
      mkdirSync(stagingDir, { recursive: true });

      const appExecutablePath = resolve(stagingDir, runtimeArtifact.fileName);
      copyFileSync(runtimeArtifact.absolutePath, appExecutablePath);

      if (updaterHelperFileName) {
        const helperPath = resolve(distDir, updaterHelperFileName);
        if (existsSync(helperPath)) {
          copyFileSync(helperPath, resolve(stagingDir, updaterHelperFileName));
        }
      }

      const msixAssets = writeMsixAssets(stagingDir, packageConfig.icon);
      const msixManifest = generateMsixManifest({
        identityName: normalizeMsixIdentityName(
          windowsConfig?.msix?.identityName ?? packageConfig.identifier,
          binaryName,
        ),
        publisher: normalizeMsixPublisher(windowsConfig?.msix?.publisher),
        publisherDisplayName: windowsConfig?.msix?.publisherDisplayName ?? appName,
        displayName: windowsConfig?.msix?.displayName ?? appName,
        description: windowsConfig?.msix?.description ?? `${appName} desktop application`,
        executableFileName: runtimeArtifact.fileName,
        version: normalizeMsixVersion(version),
        square44Logo: msixAssets.square44Logo,
        square150Logo: msixAssets.square150Logo,
      });
      writeFileSync(resolve(stagingDir, 'AppxManifest.xml'), msixManifest, 'utf8');

      const msixPath = resolve(outDir, `${binaryName}-${artifactVersion}.msix`);
      if (!runPackagingToolWithFallback(
        {
          command: 'makemsix',
          args: ['pack', '-d', stagingDir, '-p', msixPath],
        },
        {
          command: 'makeappx',
          args: ['pack', '/d', stagingDir, '/p', msixPath, '/o'],
        },
        () => {
          console.log('[volt] makemsix/makeappx not found. Install Windows SDK packaging tools to build MSIX.');
          console.log(`[volt] MSIX staging directory created: ${stagingDir}`);
        },
        '[volt] Failed to create Windows MSIX package.',
      )) {
        missingTools.push('makemsix/makeappx');
      }

      if (existsSync(msixPath)) {
        console.log(`[volt] MSIX package created: ${msixPath}`);
        if (signing) {
          signingResults.push(await signWindows(msixPath, signing));
        }
      }
    }
  }

  return missingTools;
}

export async function packageMacOS(
  appName: string,
  version: string,
  artifactVersion: string,
  binaryName: string,
  config: PackageConfig,
  outDir: string,
  runtimeArtifact: RuntimeArtifactDescriptor,
  format?: string,
  signing?: import('../../utils/signing.js').ResolvedMacOSConfig,
  signingResults: SigningArtifactResult[] = [],
): Promise<string[]> {
  const formats = format ? [format] : ['app'];
  const missingTools: string[] = [];

  for (const fmt of formats) {
    if (fmt === 'app' || fmt === 'dmg') {
      console.log('[volt] Creating macOS .app bundle...');

      const appBundlePath = resolve(outDir, `${binaryName}.app`);
      const contentsDir = resolve(appBundlePath, 'Contents');
      const macosDir = resolve(contentsDir, 'MacOS');
      const resourcesDir = resolve(contentsDir, 'Resources');

      mkdirSync(macosDir, { recursive: true });
      mkdirSync(resourcesDir, { recursive: true });

      const plist = generateInfoPlist(appName, version, binaryName, config);
      writeFileSync(resolve(contentsDir, 'Info.plist'), plist);

      const destBinary = resolve(macosDir, binaryName);
      copyFileSync(runtimeArtifact.absolutePath, destBinary);
      chmodSync(destBinary, 0o755);

      if (config.icon && existsSync(config.icon)) {
        copyFileSync(config.icon, resolve(resourcesDir, 'icon.png'));
      }

      console.log(`[volt] App bundle created: ${appBundlePath}`);

      if (signing) {
        signingResults.push(await signMacOS(appBundlePath, signing));
      }

      if (fmt === 'dmg') {
        console.log('[volt] Creating DMG...');
        const dmgPath = resolve(outDir, `${binaryName}-${artifactVersion}.dmg`);
        if (!runPackagingTool(
          'hdiutil',
          ['create', '-volname', appName, '-srcfolder', appBundlePath, '-ov', '-format', 'UDZO', dmgPath],
          () => {
            console.log('[volt] hdiutil not available. DMG creation requires macOS.');
          },
          '[volt] Failed to create DMG package.',
        )) {
          missingTools.push('hdiutil');
        }
        if (existsSync(dmgPath)) {
          console.log(`[volt] DMG created: ${dmgPath}`);
        }
      }
    }
  }

  return missingTools;
}

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
      console.log('[volt] Creating AppImage...');
      const appImageArchitecture = inferAppImageArchitecture(packageTarget, runtimeArtifact.rustTarget);

      const appDirPath = resolve(outDir, `${binaryName}.AppDir`);
      const usrBinDir = resolve(appDirPath, 'usr', 'bin');
      mkdirSync(usrBinDir, { recursive: true });

      const destBinary = resolve(usrBinDir, binaryName);
      copyFileSync(runtimeArtifact.absolutePath, destBinary);
      chmodSync(destBinary, 0o755);

      const desktopEntry = generateDesktopFile(appName, binaryName, config, 'AppRun');
      writeFileSync(resolve(appDirPath, `${binaryName}.desktop`), desktopEntry);
      writeLinuxPackageIcon(appDirPath, linuxIcon, { includeAppDirRoot: true });

      const appRun = generateAppRun(binaryName);
      writeFileSync(resolve(appDirPath, 'AppRun'), appRun, { mode: 0o755 });

      const outputPath = resolve(outDir, `${binaryName}-${artifactVersion}-${appImageArchitecture}.AppImage`);
      if (!runPackagingTool(
        'appimagetool',
        [appDirPath, outputPath],
        () => {
          console.log('[volt] appimagetool not found. Install it to create AppImages.');
          console.log(`[volt] AppDir structure created at: ${appDirPath}`);
        },
        '[volt] Failed to create AppImage package.',
      )) {
        missingTools.push('appimagetool');
      }
      if (existsSync(outputPath)) {
        console.log(`[volt] AppImage created: ${outputPath}`);
      }
    }

    if (fmt === 'deb') {
      console.log('[volt] Creating .deb package...');
      const debArchitecture = inferDebArchitecture(packageTarget, runtimeArtifact.rustTarget);

      const debDir = resolve(outDir, `${binaryName}_${artifactVersion}_${debArchitecture}`);
      const debBinDir = resolve(debDir, 'usr', 'bin');
      const debControlDir = resolve(debDir, 'DEBIAN');
      const debDesktopDir = resolve(debDir, 'usr', 'share', 'applications');

      mkdirSync(debBinDir, { recursive: true });
      mkdirSync(debControlDir, { recursive: true });
      mkdirSync(debDesktopDir, { recursive: true });

      const destBinary = resolve(debBinDir, binaryName);
      copyFileSync(runtimeArtifact.absolutePath, destBinary);
      chmodSync(destBinary, 0o755);

      const control = [
        `Package: ${binaryName}`,
        `Version: ${debControlVersion}`,
        'Section: utils',
        'Priority: optional',
        `Architecture: ${debArchitecture}`,
        `Maintainer: ${appName} Developer`,
        `Description: ${appName}`,
        `  Desktop application built with Volt framework.`,
        '',
      ].join('\n');
      writeFileSync(resolve(debControlDir, 'control'), control);

      const desktopEntry = generateDesktopFile(appName, binaryName, config);
      writeFileSync(resolve(debDesktopDir, `${binaryName}.desktop`), desktopEntry);
      writeLinuxPackageIcon(debDir, linuxIcon);

      const debPath = resolve(outDir, `${binaryName}_${artifactVersion}_${debArchitecture}.deb`);
      if (!runPackagingTool(
        'dpkg-deb',
        ['--build', debDir, debPath],
        () => {
          console.log('[volt] dpkg-deb not found. Install dpkg to create .deb packages.');
        },
        '[volt] Failed to create deb package.',
      )) {
        missingTools.push('dpkg-deb');
      }
      if (existsSync(debPath)) {
        console.log(`[volt] Deb package created: ${debPath}`);
      }
    }
  }

  return missingTools;
}

interface MsixAssetPaths {
  square44Logo: string;
  square150Logo: string;
}

const PLACEHOLDER_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO7Yz0QAAAAASUVORK5CYII=',
  'base64',
);
const PLACEHOLDER_LINUX_SVG = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256" role="img" aria-label="Volt app">
  <rect width="256" height="256" rx="48" fill="#101828" />
  <path d="M76 56h33l25 93 23-93h23l-36 144h-30L76 56Z" fill="#ffffff" />
</svg>
`;
const SUPPORTED_LINUX_ICON_EXTENSIONS = new Set(['.png', '.svg', '.xpm']);

function writeMsixAssets(stagingDir: string, iconPath: string | undefined): MsixAssetPaths {
  const assetsDir = resolve(stagingDir, 'Assets');
  mkdirSync(assetsDir, { recursive: true });

  const square44 = resolve(assetsDir, 'Square44x44Logo.png');
  const square150 = resolve(assetsDir, 'Square150x150Logo.png');

  if (iconPath && existsSync(iconPath) && iconPath.toLowerCase().endsWith('.png')) {
    copyFileSync(iconPath, square44);
    copyFileSync(iconPath, square150);
  } else {
    writeFileSync(square44, PLACEHOLDER_PNG);
    writeFileSync(square150, PLACEHOLDER_PNG);
  }

  return {
    square44Logo: 'Assets/Square44x44Logo.png',
    square150Logo: 'Assets/Square150x150Logo.png',
  };
}

interface LinuxPackageIcon {
  fileName: string;
  sourcePath: string | null;
  placeholderContents: string | null;
  themeDirectory: string[];
  message: string | null;
}

function resolveLinuxPackageIcon(binaryName: string, iconPath: string | undefined): LinuxPackageIcon {
  if (iconPath && existsSync(iconPath)) {
    const extension = extname(iconPath).toLowerCase();
    if (SUPPORTED_LINUX_ICON_EXTENSIONS.has(extension)) {
      return {
        fileName: `${binaryName}${extension}`,
        sourcePath: iconPath,
        placeholderContents: null,
        themeDirectory: extension === '.svg'
          ? ['usr', 'share', 'icons', 'hicolor', 'scalable', 'apps']
          : ['usr', 'share', 'icons', 'hicolor', '256x256', 'apps'],
        message: null,
      };
    }

    return {
      fileName: `${binaryName}.svg`,
      sourcePath: null,
      placeholderContents: PLACEHOLDER_LINUX_SVG,
      themeDirectory: ['usr', 'share', 'icons', 'hicolor', 'scalable', 'apps'],
      message: `Linux packaging supports .png, .svg, or .xpm icons. Using a generated placeholder icon instead of "${iconPath}".`,
    };
  }

  return {
    fileName: `${binaryName}.svg`,
    sourcePath: null,
    placeholderContents: PLACEHOLDER_LINUX_SVG,
    themeDirectory: ['usr', 'share', 'icons', 'hicolor', 'scalable', 'apps'],
    message: iconPath
      ? `Configured icon "${iconPath}" was not found. Using a generated placeholder icon for Linux packaging.`
      : 'No application icon configured. Using a generated placeholder icon for Linux packaging.',
  };
}

function writeLinuxPackageIcon(
  rootDir: string,
  icon: LinuxPackageIcon,
  options: { includeAppDirRoot?: boolean } = {},
): void {
  const themeDir = resolve(rootDir, ...icon.themeDirectory);
  mkdirSync(themeDir, { recursive: true });
  writeLinuxIconFile(resolve(themeDir, icon.fileName), icon);

  if (options.includeAppDirRoot) {
    writeLinuxIconFile(resolve(rootDir, icon.fileName), icon);
  }
}

function writeLinuxIconFile(path: string, icon: LinuxPackageIcon): void {
  if (icon.sourcePath) {
    copyFileSync(icon.sourcePath, path);
    return;
  }
  writeFileSync(path, icon.placeholderContents ?? PLACEHOLDER_LINUX_SVG, 'utf8');
}

function normalizeMsixIdentityName(candidate: string | undefined, binaryName: string): string {
  const fallback = `com.volt.${binaryName}`.replace(/[^A-Za-z0-9.]/g, '.');
  if (!candidate || candidate.trim().length === 0) {
    return fallback;
  }

  const normalized = candidate
    .trim()
    .replace(/[^A-Za-z0-9.]/g, '.')
    .replace(/\.{2,}/g, '.')
    .replace(/^\.+|\.+$/g, '');

  if (normalized.length === 0) {
    return fallback;
  }

  if (!/^[A-Za-z]/.test(normalized)) {
    return `app.${normalized}`;
  }

  return normalized;
}

function normalizeMsixPublisher(candidate: string | undefined): string {
  if (!candidate || candidate.trim().length === 0) {
    return 'CN=VoltDeveloper';
  }
  const normalized = candidate.trim();
  if (/^CN=/i.test(normalized)) {
    return normalized;
  }
  return `CN=${normalized}`;
}
