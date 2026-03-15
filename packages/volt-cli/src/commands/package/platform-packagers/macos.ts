import { chmodSync, copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

import { signMacOS } from '../../../utils/signing.js';
import type { SigningArtifactResult } from '../../../utils/signing.js';
import type { RuntimeArtifactDescriptor } from '../../../utils/runtime-artifact.js';
import { runPackagingTool } from '../helpers.js';
import { generateInfoPlist } from '../templates.js';
import type { PackageConfig } from '../types.js';

import { copySidecarFiles } from './shared.js';

export async function packageMacOS(
  appName: string,
  version: string,
  artifactVersion: string,
  binaryName: string,
  config: PackageConfig,
  outDir: string,
  runtimeArtifact: RuntimeArtifactDescriptor,
  format?: string,
  signing?: import('../../../utils/signing.js').ResolvedMacOSConfig,
  signingResults: SigningArtifactResult[] = [],
): Promise<string[]> {
  const formats = format ? [format] : ['app'];
  const missingTools: string[] = [];

  for (const fmt of formats) {
    if (fmt !== 'app' && fmt !== 'dmg') continue;

    console.log('[volt] Creating macOS .app bundle...');
    const appBundlePath = resolve(outDir, `${binaryName}.app`);
    const contentsDir = resolve(appBundlePath, 'Contents');
    const macosDir = resolve(contentsDir, 'MacOS');
    const resourcesDir = resolve(contentsDir, 'Resources');

    mkdirSync(macosDir, { recursive: true });
    mkdirSync(resourcesDir, { recursive: true });
    writeFileSync(
      resolve(contentsDir, 'Info.plist'),
      generateInfoPlist(appName, version, binaryName, config),
    );

    const destBinary = resolve(macosDir, binaryName);
    copyFileSync(runtimeArtifact.absolutePath, destBinary);
    chmodSync(destBinary, 0o755);
    copySidecarFiles(dirname(runtimeArtifact.absolutePath), macosDir);

    if (config.icon && existsSync(config.icon)) {
      copyFileSync(config.icon, resolve(resourcesDir, 'icon.png'));
    }

    console.log(`[volt] App bundle created: ${appBundlePath}`);
    if (signing) {
      signingResults.push(await signMacOS(appBundlePath, signing));
    }

    if (fmt === 'dmg') {
      const dmgPath = resolve(outDir, `${binaryName}-${artifactVersion}.dmg`);
      console.log('[volt] Creating DMG...');
      if (
        !runPackagingTool(
          'hdiutil',
          [
            'create',
            '-volname',
            appName,
            '-srcfolder',
            appBundlePath,
            '-ov',
            '-format',
            'UDZO',
            dmgPath,
          ],
          () => {
            console.log('[volt] hdiutil not available. DMG creation requires macOS.');
          },
          '[volt] Failed to create DMG package.',
        )
      ) {
        missingTools.push('hdiutil');
      }
      if (existsSync(dmgPath)) {
        console.log(`[volt] DMG created: ${dmgPath}`);
      }
    }
  }

  return missingTools;
}
