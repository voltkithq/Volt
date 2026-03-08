import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import type { VoltConfig } from 'voltkit';
import { loadConfig } from '../../utils/config.js';
import { isToolAvailable } from '../../utils/signing.js';
import type { SignSetupContext, SignSetupOptions, SignSetupPlatform, SignSetupWindowsProvider } from './types.js';

const WINDOWS_PROVIDER_ALIASES: Readonly<Record<string, SignSetupWindowsProvider>> = {
  local: 'local',
  azuretrustedsigning: 'azureTrustedSigning',
  azure_trusted_signing: 'azureTrustedSigning',
  azure: 'azureTrustedSigning',
  digicertkeylocker: 'digicertKeyLocker',
  digicert_keylocker: 'digicertKeyLocker',
  digicert: 'digicertKeyLocker',
  keylocker: 'digicertKeyLocker',
};

function normalizeSignSetupPlatform(raw: string | undefined): SignSetupPlatform {
  if (!raw) {
    if (process.platform === 'darwin') {
      return 'darwin';
    }
    if (process.platform === 'win32') {
      return 'win32';
    }
    return 'all';
  }

  const normalized = raw.trim().toLowerCase();
  if (normalized === 'darwin' || normalized === 'macos' || normalized === 'mac') {
    return 'darwin';
  }
  if (normalized === 'win32' || normalized === 'windows') {
    return 'win32';
  }
  if (normalized === 'all') {
    return 'all';
  }

  throw new Error(
    `[volt] Invalid --platform value "${raw}". Expected one of: darwin, win32, all.`,
  );
}

function normalizeWindowsProvider(raw: string | undefined): SignSetupWindowsProvider {
  const normalized = raw?.trim().toLowerCase();
  if (!normalized) {
    return 'local';
  }

  const mapped = WINDOWS_PROVIDER_ALIASES[normalized];
  if (mapped) {
    return mapped;
  }

  throw new Error(
    `[volt] Invalid --windows-provider value "${raw}". Expected one of: local, azureTrustedSigning, digicertKeyLocker.`,
  );
}

function resolveSignSetupContext(config: VoltConfig, options: SignSetupOptions): SignSetupContext {
  const platform = normalizeSignSetupPlatform(options.platform);
  const configProvider = config.package?.signing?.windows?.provider;
  const windowsProvider = normalizeWindowsProvider(
    options.windowsProvider
      ?? process.env['VOLT_WIN_SIGNING_PROVIDER']
      ?? configProvider,
  );

  return {
    platform,
    windowsProvider,
  };
}

function buildMacSection(): string[] {
  return [
    '# macOS signing + notarization',
    'VOLT_MACOS_SIGNING_IDENTITY=',
    'VOLT_APPLE_TEAM_ID=',
    'VOLT_APPLE_ID=',
    'VOLT_APPLE_PASSWORD=',
    '# Optional CI-only certificate import',
    'VOLT_MACOS_CERTIFICATE=',
    'VOLT_MACOS_CERTIFICATE_PASSWORD=',
    '',
  ];
}

function buildWindowsLocalSection(): string[] {
  return [
    '# Windows local certificate signing',
    'VOLT_WIN_SIGNING_PROVIDER=local',
    'VOLT_WIN_CERTIFICATE=',
    'VOLT_WIN_CERTIFICATE_PASSWORD=',
    '# Optional overrides',
    'VOLT_WIN_TIMESTAMP_URL=',
    '',
  ];
}

function buildWindowsAzureSection(): string[] {
  return [
    '# Windows Azure Trusted Signing',
    'VOLT_WIN_SIGNING_PROVIDER=azureTrustedSigning',
    'VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH=',
    'VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH=',
    '# Optional provider metadata',
    'VOLT_AZURE_TRUSTED_SIGNING_ENDPOINT=',
    'VOLT_AZURE_TRUSTED_SIGNING_ACCOUNT_NAME=',
    'VOLT_AZURE_TRUSTED_SIGNING_CERT_PROFILE=',
    'VOLT_AZURE_TRUSTED_SIGNING_CORRELATION_ID=',
    '# Optional overrides',
    'VOLT_WIN_TIMESTAMP_URL=',
    '',
  ];
}

function buildWindowsDigiCertSection(): string[] {
  return [
    '# Windows DigiCert KeyLocker',
    'VOLT_WIN_SIGNING_PROVIDER=digicertKeyLocker',
    'VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS=',
    '# Optional provider metadata',
    'VOLT_DIGICERT_KEYLOCKER_CERT_FINGERPRINT=',
    'VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH=smctl',
    'VOLT_DIGICERT_KEYLOCKER_TIMESTAMP_URL=',
    '# Optional common override',
    'VOLT_WIN_TIMESTAMP_URL=',
    '',
  ];
}

function buildSigningEnvTemplate(context: SignSetupContext): string {
  const lines: string[] = [
    '# Volt signing bootstrap template',
    '# Fill values, then load them into your shell/CI secrets before running `volt package`.',
    '',
  ];

  if (context.platform === 'darwin' || context.platform === 'all') {
    lines.push(...buildMacSection());
  }

  if (context.platform === 'win32' || context.platform === 'all') {
    if (context.windowsProvider === 'azureTrustedSigning') {
      lines.push(...buildWindowsAzureSection());
    } else if (context.windowsProvider === 'digicertKeyLocker') {
      lines.push(...buildWindowsDigiCertSection());
    } else {
      lines.push(...buildWindowsLocalSection());
    }
  }

  return `${lines.join('\n').trimEnd()}\n`;
}

function collectToolStatus(context: SignSetupContext): string[] {
  const lines: string[] = [];

  if (context.platform === 'darwin' || context.platform === 'all') {
    const codesign = isToolAvailable('codesign');
    const xcrun = isToolAvailable('xcrun');
    lines.push(`[volt] macOS tool check: codesign=${codesign ? 'ok' : 'missing'}, xcrun=${xcrun ? 'ok' : 'missing'}`);
  }

  if (context.platform === 'win32' || context.platform === 'all') {
    if (context.windowsProvider === 'local') {
      const signtool = isToolAvailable('signtool');
      const osslsigncode = isToolAvailable('osslsigncode');
      lines.push(
        `[volt] Windows tool check (local): signtool=${signtool ? 'ok' : 'missing'}, osslsigncode=${osslsigncode ? 'ok' : 'missing'}`,
      );
    } else if (context.windowsProvider === 'azureTrustedSigning') {
      const signtool = isToolAvailable('signtool');
      lines.push(`[volt] Windows tool check (azureTrustedSigning): signtool=${signtool ? 'ok' : 'missing'}`);
    } else {
      const smctl = isToolAvailable('smctl');
      lines.push(`[volt] Windows tool check (digicertKeyLocker): smctl=${smctl ? 'ok' : 'missing'}`);
    }
  }

  return lines;
}

export async function signSetupCommand(options: SignSetupOptions): Promise<void> {
  const cwd = process.cwd();
  const config = await loadConfig(cwd, { strict: false, commandName: 'sign setup' });
  const context = resolveSignSetupContext(config, options);
  const template = buildSigningEnvTemplate(context);
  const outputPath = resolve(cwd, options.output ?? '.env.signing');

  if (options.printOnly !== true) {
    if (existsSync(outputPath) && options.force !== true) {
      console.error(
        `[volt] Refusing to overwrite existing file: ${outputPath}. Pass --force to overwrite or use --output.`,
      );
      process.exit(1);
    }
    mkdirSync(dirname(outputPath), { recursive: true });
    writeFileSync(outputPath, template, 'utf8');
    console.log(`[volt] Signing template written: ${outputPath}`);
  }

  if (options.print || options.printOnly) {
    console.log(template.trimEnd());
  }

  console.log(`[volt] Setup context: platform=${context.platform}, windowsProvider=${context.windowsProvider}`);
  for (const line of collectToolStatus(context)) {
    console.log(line);
  }
  console.log('[volt] Next: fill values, load environment variables, then run `volt package`.');
}

export const __testOnly = {
  normalizeSignSetupPlatform,
  normalizeWindowsProvider,
  resolveSignSetupContext,
  buildSigningEnvTemplate,
  collectToolStatus,
};
