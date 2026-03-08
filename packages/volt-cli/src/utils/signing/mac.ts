import { randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { existsSync, unlinkSync, writeFileSync } from 'node:fs';
import { runSigningCommand } from './command.js';
import { notarizeMacOS } from './notarization.js';
import { isToolAvailable } from './tooling.js';
import type { ResolvedMacOSConfig, SigningArtifactResult } from './types.js';

/**
 * Import a base64-encoded .p12 certificate into a temporary macOS keychain.
 * Used in CI environments where the certificate is passed as an env var.
 * Returns the path to the temporary keychain.
 */
export function importCertificateToKeychain(base64Cert: string, password: string): string {
  const keychainName = `volt-signing-${randomBytes(8).toString('hex')}.keychain-db`;
  const keychainPath = resolve(process.env['RUNNER_TEMP'] ?? '/tmp', keychainName);
  const keychainPassword = randomBytes(16).toString('hex');
  const certPath = resolve(dirname(keychainPath), `volt-signing-cert-${randomBytes(8).toString('hex')}.p12`);

  const certBuffer = Buffer.from(base64Cert, 'base64');
  writeFileSync(certPath, certBuffer, { mode: 0o600 });

  try {
    execFileSync('security', ['create-keychain', '-p', keychainPassword, keychainPath], {
      stdio: 'pipe',
    });
    execFileSync('security', ['set-keychain-settings', '-t', '3600', '-u', keychainPath], {
      stdio: 'pipe',
    });
    execFileSync('security', ['unlock-keychain', '-p', keychainPassword, keychainPath], {
      stdio: 'pipe',
    });
    execFileSync(
      'security',
      [
        'import',
        certPath,
        '-k',
        keychainPath,
        '-P',
        password,
        '-T',
        '/usr/bin/codesign',
        '-T',
        '/usr/bin/security',
      ],
      { stdio: 'pipe' },
    );

    const existingKeychains = execFileSync('security', ['list-keychains', '-d', 'user'], {
      encoding: 'utf-8',
    })
      .trim()
      .split('\n')
      .map((keychain) => keychain.trim().replace(/"/g, ''))
      .filter(Boolean);
    execFileSync(
      'security',
      ['list-keychains', '-d', 'user', '-s', keychainPath, ...existingKeychains],
      { stdio: 'pipe' },
    );
    execFileSync(
      'security',
      [
        'set-key-partition-list',
        '-S',
        'apple-tool:,apple:,codesign:',
        '-s',
        '-k',
        keychainPassword,
        keychainPath,
      ],
      { stdio: 'pipe' },
    );

    return keychainPath;
  } finally {
    if (existsSync(certPath)) {
      unlinkSync(certPath);
    }
  }
}

/**
 * Remove a temporary keychain created by importCertificateToKeychain.
 */
export function cleanupKeychain(keychainPath: string): void {
  try {
    execFileSync('security', ['delete-keychain', keychainPath], { stdio: 'pipe' });
  } catch {
    // Best-effort cleanup.
  }
}

function hasNotarizationCredentials(config: ResolvedMacOSConfig): boolean {
  return Boolean(config.appleId && config.applePassword && config.teamId);
}

/**
 * Sign a macOS .app bundle and optionally notarize it.
 */
export async function signMacOS(
  appBundlePath: string,
  config: ResolvedMacOSConfig,
): Promise<SigningArtifactResult> {
  const startedAt = new Date().toISOString();

  if (!isToolAvailable('codesign')) {
    throw new Error('codesign not found. macOS code signing requires Xcode Command Line Tools.');
  }

  let tempKeychainPath: string | undefined;
  let notarized = false;

  try {
    if (config.certificate) {
      console.log('[volt] Importing certificate to temporary keychain...');
      tempKeychainPath = importCertificateToKeychain(config.certificate, config.certificatePassword ?? '');
    }

    console.log(`[volt] Signing ${appBundlePath}...`);
    const codesignArgs = [
      '--deep',
      '--force',
      '--options',
      'runtime',
      '--timestamp',
      '--sign',
      config.identity,
    ];
    if (config.entitlements) {
      codesignArgs.push('--entitlements', config.entitlements);
    }
    if (tempKeychainPath) {
      codesignArgs.push('--keychain', tempKeychainPath);
    }
    codesignArgs.push(appBundlePath);
    runSigningCommand('codesign', codesignArgs, {
      description: 'codesign sign',
    });

    console.log('[volt] Verifying signature...');
    runSigningCommand('codesign', ['--verify', '--deep', '--strict', appBundlePath], {
      description: 'codesign verify',
    });

    if (config.notarize) {
      if (!hasNotarizationCredentials(config)) {
        console.warn(
          '[volt] Skipping notarization: VOLT_APPLE_ID, VOLT_APPLE_PASSWORD, and VOLT_APPLE_TEAM_ID are all required.',
        );
      } else {
        await notarizeMacOS(appBundlePath, config);
        notarized = true;
      }
    }

    console.log('[volt] macOS signing complete.');
    return {
      platform: 'darwin',
      provider: 'apple',
      tool: 'codesign',
      targetPath: appBundlePath,
      signed: true,
      notarized,
      startedAt,
      finishedAt: new Date().toISOString(),
      details: {
        identity: config.identity,
        teamId: config.teamId,
        notarize: config.notarize,
      },
    };
  } finally {
    if (tempKeychainPath) {
      console.log('[volt] Cleaning up temporary keychain...');
      cleanupKeychain(tempKeychainPath);
    }
  }
}
