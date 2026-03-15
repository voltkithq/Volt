import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { extname, resolve } from 'node:path';

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

export interface MsixAssetPaths {
  square44Logo: string;
  square150Logo: string;
}

export interface LinuxPackageIcon {
  fileName: string;
  sourcePath: string | null;
  placeholderContents: string | null;
  themeDirectory: string[];
  message: string | null;
}

export function writeMsixAssets(stagingDir: string, iconPath: string | undefined): MsixAssetPaths {
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

export function resolveLinuxPackageIcon(
  binaryName: string,
  iconPath: string | undefined,
): LinuxPackageIcon {
  if (iconPath && existsSync(iconPath)) {
    const extension = extname(iconPath).toLowerCase();
    if (SUPPORTED_LINUX_ICON_EXTENSIONS.has(extension)) {
      return {
        fileName: `${binaryName}${extension}`,
        sourcePath: iconPath,
        placeholderContents: null,
        themeDirectory:
          extension === '.svg'
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

export function writeLinuxPackageIcon(
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

export function normalizeMsixIdentityName(
  candidate: string | undefined,
  binaryName: string,
): string {
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

export function normalizeMsixPublisher(candidate: string | undefined): string {
  if (!candidate || candidate.trim().length === 0) {
    return 'CN=VoltDeveloper';
  }
  const normalized = candidate.trim();
  return /^CN=/i.test(normalized) ? normalized : `CN=${normalized}`;
}

function writeLinuxIconFile(path: string, icon: LinuxPackageIcon): void {
  if (icon.sourcePath) {
    copyFileSync(icon.sourcePath, path);
    return;
  }
  writeFileSync(path, icon.placeholderContents ?? PLACEHOLDER_LINUX_SVG, 'utf8');
}
