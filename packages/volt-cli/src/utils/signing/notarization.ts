import { existsSync, unlinkSync } from 'node:fs';
import { runSigningCommand } from './command.js';
import { isToolAvailable } from './tooling.js';
import type { ResolvedMacOSConfig } from './types.js';

/**
 * Notarize a macOS .app bundle using xcrun notarytool.
 */
export async function notarizeMacOS(appBundlePath: string, config: ResolvedMacOSConfig): Promise<void> {
  if (!isToolAvailable('xcrun')) {
    throw new Error('xcrun not found. Notarization requires Xcode Command Line Tools.');
  }

  console.log('[volt] Notarizing (this may take several minutes)...');

  const zipPath = `${appBundlePath}.zip`;
  runSigningCommand('ditto', ['-c', '-k', '--keepParent', appBundlePath, zipPath], {
    description: 'ditto archive',
  });

  try {
    runSigningCommand(
      'xcrun',
      [
        'notarytool',
        'submit',
        zipPath,
        '--apple-id',
        config.appleId!,
        '--password',
        config.applePassword!,
        '--team-id',
        config.teamId!,
        '--wait',
      ],
      { description: 'xcrun notarytool submit' },
    );

    console.log('[volt] Stapling notarization ticket...');
    runSigningCommand('xcrun', ['stapler', 'staple', appBundlePath], {
      description: 'xcrun stapler staple',
    });

    console.log('[volt] Notarization complete.');
  } finally {
    if (existsSync(zipPath)) {
      unlinkSync(zipPath);
    }
  }
}
