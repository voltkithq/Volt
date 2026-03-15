import { runSigningCommand } from '../command.js';
import { isToolAvailable } from '../tooling.js';
import type { VerificationTool } from './types.js';

export function selectVerificationTool(): VerificationTool | null {
  if (process.platform === 'win32' && isToolAvailable('signtool')) {
    return 'signtool';
  }
  if (isToolAvailable('osslsigncode')) {
    return 'osslsigncode';
  }
  if (isToolAvailable('signtool')) {
    return 'signtool';
  }
  return null;
}

export function verifyWindowsSignature(exePath: string, preferredTool?: VerificationTool): void {
  const tool = preferredTool ?? selectVerificationTool();
  if (!tool) {
    throw new Error(
      'Signature verification requires signtool.exe or osslsigncode. Install one of them and ensure it is on PATH.',
    );
  }

  console.log('[volt] Verifying signature...');
  if (tool === 'signtool') {
    runSigningCommand('signtool', ['verify', '/pa', exePath], {
      description: 'signtool verify',
    });
    return;
  }

  runSigningCommand('osslsigncode', ['verify', exePath], {
    description: 'osslsigncode verify',
  });
}
