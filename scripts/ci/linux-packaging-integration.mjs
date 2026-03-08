import { execFileSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const WORKSPACE_ROOT = resolve(SCRIPT_DIR, '..', '..');
const APP_DIR = resolve(WORKSPACE_ROOT, 'examples', 'hello-world');
const DIST_PACKAGE_DIR = resolve(APP_DIR, 'dist-package');
const BINARY_NAME = process.env.VOLT_PACKAGE_BINARY_NAME ?? 'hello-world';
const VERSION = process.env.VOLT_PACKAGE_VERSION ?? '0.1.0';
const TARGET = process.env.VOLT_PACKAGE_TARGET ?? 'x86_64-unknown-linux-gnu';
const EXPECTED_APPIMAGE_ARCH = process.env.VOLT_EXPECTED_APPIMAGE_ARCH ?? 'x86_64';
const EXPECTED_DEB_ARCH = process.env.VOLT_EXPECTED_DEB_ARCH ?? 'amd64';

function main() {
  ensureLinux();
  ensureTooling();
  runBuild();
  runPackaging();
  assertArtifactsExist();
  console.log('[packaging-integration] Linux packaging integration passed.');
}

function ensureLinux() {
  if (process.platform !== 'linux') {
    throw new Error(
      `[packaging-integration] This script must run on Linux. Current platform: ${process.platform}`,
    );
  }
}

function ensureTooling() {
  exec('appimagetool', ['--appimage-version'], WORKSPACE_ROOT);
  exec('dpkg-deb', ['--version'], WORKSPACE_ROOT);
}

function runBuild() {
  const voltCliEntry = resolve(WORKSPACE_ROOT, 'packages', 'volt-cli', 'dist', 'index.js');
  if (!existsSync(voltCliEntry)) {
    throw new Error(
      `[packaging-integration] Missing CLI build output: ${voltCliEntry}. Run \`pnpm --filter @voltkit/volt-cli run build\` first.`,
    );
  }

  exec('node', [voltCliEntry, 'build', '--target', TARGET], APP_DIR);
}

function runPackaging() {
  const voltCliEntry = resolve(WORKSPACE_ROOT, 'packages', 'volt-cli', 'dist', 'index.js');
  if (!existsSync(voltCliEntry)) {
    throw new Error(
      `[packaging-integration] Missing CLI build output: ${voltCliEntry}. Run \`pnpm --filter @voltkit/volt-cli run build\` first.`,
    );
  }

  exec('node', [voltCliEntry, 'package', '--target', TARGET], APP_DIR);
}

function assertArtifactsExist() {
  const expectedAppImage = resolve(
    DIST_PACKAGE_DIR,
    `${BINARY_NAME}-${VERSION}-${EXPECTED_APPIMAGE_ARCH}.AppImage`,
  );
  const expectedDeb = resolve(DIST_PACKAGE_DIR, `${BINARY_NAME}_${VERSION}_${EXPECTED_DEB_ARCH}.deb`);

  assertFileNonEmpty(expectedAppImage, 'AppImage');
  assertFileNonEmpty(expectedDeb, 'deb');

  console.log(`[packaging-integration] Verified AppImage: ${expectedAppImage}`);
  console.log(`[packaging-integration] Verified deb: ${expectedDeb}`);
}

function assertFileNonEmpty(path, label) {
  if (!existsSync(path)) {
    throw new Error(`[packaging-integration] Missing ${label} artifact: ${path}`);
  }
  const size = statSync(path).size;
  if (size <= 0) {
    throw new Error(`[packaging-integration] ${label} artifact is empty: ${path}`);
  }
}

function exec(command, args, cwd) {
  execFileSync(command, args, {
    cwd,
    stdio: 'inherit',
    env: process.env,
  });
}

main();
