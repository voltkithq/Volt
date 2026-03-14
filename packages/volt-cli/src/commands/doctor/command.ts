import { loadConfig } from '../../utils/config.js';
import { resolveSigningConfig, isToolAvailable } from '../../utils/signing.js';
import { normalizePackagePlatform } from '../../utils/runtime-artifact.js';
import { parsePackageConfig, validateRequestedPackageFormat } from '../package/config.js';
import type { PackageConfig } from '../package/types.js';
import { runBuildPreflight, runPackagePreflight } from '../../utils/preflight.js';

export interface DoctorOptions {
  target?: string;
  format?: string;
  json?: boolean;
}

export type DoctorPlatform = 'win32' | 'darwin' | 'linux';
export type DoctorCheckStatus = 'pass' | 'warn' | 'fail';

export interface DoctorCheckResult {
  id: string;
  status: DoctorCheckStatus;
  title: string;
  details: string;
}

export interface DoctorReport {
  target: DoctorPlatform;
  formats: string[];
  checks: DoctorCheckResult[];
  summary: {
    pass: number;
    warn: number;
    fail: number;
  };
}

interface DoctorDeps {
  isToolAvailable: (toolName: string) => boolean;
  env: NodeJS.ProcessEnv;
}

export async function doctorCommand(options: DoctorOptions): Promise<void> {
  const cwd = process.cwd();
  const config = await loadConfig(cwd, { strict: false, commandName: 'doctor' });
  const platform = normalizePackagePlatform(options.target);
  const requestedFormat = validateRequestedPackageFormat(platform, options.format);
  if (options.format && !requestedFormat) {
    const supported = resolveDoctorFormats(platform, undefined).join(', ');
    console.error(
      `[volt:doctor] Unsupported package format "${options.format}" for platform "${platform}". Supported formats: ${supported}.`,
    );
    process.exit(1);
  }

  const packageConfig = parsePackageConfig(
    (config as unknown as Record<string, unknown>)['package'],
    config.name,
  );
  const formats = resolveDoctorFormats(platform, requestedFormat);
  const checks = collectDoctorChecks(
    {
      platform,
      formats,
      packageConfig,
    },
    {
      isToolAvailable,
      env: process.env,
    },
  );

  // Also run the shared preflight checks and merge results
  const buildPreflight = runBuildPreflight(cwd, config, { target: options.target });
  for (const error of buildPreflight.errors) {
    if (!checks.some((c) => c.id === error.id)) {
      checks.push({
        id: error.id,
        status: 'fail',
        title: error.message,
        details: error.fix ?? '',
      });
    }
  }
  const packagePreflight = runPackagePreflight(cwd, platform, { format: requestedFormat });
  for (const error of packagePreflight.errors) {
    if (!checks.some((c) => c.id === error.id)) {
      checks.push({
        id: error.id,
        status: 'fail',
        title: error.message,
        details: error.fix ?? '',
      });
    }
  }
  for (const warning of [...buildPreflight.warnings, ...packagePreflight.warnings]) {
    if (!checks.some((c) => c.id === warning.id)) {
      checks.push({
        id: warning.id,
        status: 'warn',
        title: warning.message,
        details: '',
      });
    }
  }
  const report: DoctorReport = {
    target: platform,
    formats,
    checks,
    summary: summarizeDoctorChecks(checks),
  };

  if (options.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    printDoctorReport(report);
  }

  if (report.summary.fail > 0) {
    process.exit(1);
  }
}

export function resolveDoctorFormats(
  platform: DoctorPlatform,
  requestedFormat: string | undefined,
): string[] {
  if (requestedFormat) {
    return [requestedFormat];
  }
  if (platform === 'win32') {
    return ['nsis'];
  }
  if (platform === 'darwin') {
    return ['app'];
  }
  return ['appimage', 'deb'];
}

export function collectDoctorChecks(
  context: {
    platform: DoctorPlatform;
    formats: readonly string[];
    packageConfig: PackageConfig;
  },
  deps: DoctorDeps,
): DoctorCheckResult[] {
  const checks: DoctorCheckResult[] = [];

  checks.push(
    createToolCheck(
      'tool.cargo',
      'Rust toolchain (`cargo`)',
      'cargo',
      deps.isToolAvailable,
      'required for `volt build` and packaging workflows',
    ),
  );

  checks.push(
    createToolCheck(
      'tool.rustc',
      'Rust compiler (`rustc`)',
      'rustc',
      deps.isToolAvailable,
      'required for native runtime compilation',
    ),
  );

  if (context.platform === 'win32') {
    if (context.formats.includes('nsis')) {
      checks.push(
        createToolCheck(
          'pkg.win.nsis',
          'NSIS packager (`makensis`)',
          'makensis',
          deps.isToolAvailable,
          'required for NSIS installer output',
        ),
      );
    }

    if (context.formats.includes('msix')) {
      const hasMakemsix = deps.isToolAvailable('makemsix');
      const hasMakeappx = deps.isToolAvailable('makeappx');
      checks.push({
        id: 'pkg.win.msix',
        status: hasMakemsix || hasMakeappx ? 'pass' : 'fail',
        title: 'MSIX packager (`makemsix` or `makeappx`)',
        details: hasMakemsix || hasMakeappx
          ? 'MSIX packaging tools detected'
          : 'install Windows SDK tooling (`makemsix` or `makeappx`) to build MSIX packages',
      });
    }
  }

  if (context.platform === 'darwin' && context.formats.includes('dmg')) {
    checks.push(
      createToolCheck(
        'pkg.mac.dmg',
        'DMG tool (`hdiutil`)',
        'hdiutil',
        deps.isToolAvailable,
        'required for `.dmg` output',
      ),
    );
  }

  if (context.platform === 'linux') {
    if (context.formats.includes('appimage')) {
      checks.push(
        createToolCheck(
          'pkg.linux.appimage',
          'AppImage tool (`appimagetool`)',
          'appimagetool',
          deps.isToolAvailable,
          'required for `.AppImage` output',
        ),
      );
    }
    if (context.formats.includes('deb')) {
      checks.push(
        createToolCheck(
          'pkg.linux.deb',
          'Debian packager (`dpkg-deb`)',
          'dpkg-deb',
          deps.isToolAvailable,
          'required for `.deb` output',
        ),
      );
    }
  }

  if (context.platform === 'win32') {
    checks.push(...collectWindowsSigningChecks(context.packageConfig, deps));
  }
  if (context.platform === 'darwin') {
    checks.push(...collectMacSigningChecks(context.packageConfig, deps));
  }

  return checks;
}

export function summarizeDoctorChecks(checks: readonly DoctorCheckResult[]): DoctorReport['summary'] {
  return {
    pass: checks.filter((check) => check.status === 'pass').length,
    warn: checks.filter((check) => check.status === 'warn').length,
    fail: checks.filter((check) => check.status === 'fail').length,
  };
}

function collectWindowsSigningChecks(
  packageConfig: PackageConfig,
  deps: DoctorDeps,
): DoctorCheckResult[] {
  const checks: DoctorCheckResult[] = [];
  const signing = resolveSigningConfig(packageConfig, 'win32')?.windows;
  if (!signing) {
    checks.push({
      id: 'signing.win.disabled',
      status: 'warn',
      title: 'Windows signing configuration',
      details: 'not configured (set `package.signing.windows` or related VOLT_WIN_* env vars)',
    });
    return checks;
  }

  if (signing.provider === 'azureTrustedSigning') {
    checks.push(
      createToolCheck(
        'signing.win.azure.signtool',
        'Azure Trusted Signing tool (`signtool`)',
        'signtool',
        deps.isToolAvailable,
        'required for Azure Trusted Signing flow',
      ),
    );
    checks.push({
      id: 'signing.win.azure.dlib',
      status: signing.azureTrustedSigning?.dlibPath ? 'pass' : 'fail',
      title: 'Azure Trusted Signing dlib path',
      details: signing.azureTrustedSigning?.dlibPath
        ? `configured: ${signing.azureTrustedSigning.dlibPath}`
        : 'missing `VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH` or `package.signing.windows.azureTrustedSigning.dlibPath`',
    });
    checks.push({
      id: 'signing.win.azure.metadata',
      status: signing.azureTrustedSigning?.metadataPath ? 'pass' : 'fail',
      title: 'Azure Trusted Signing metadata path',
      details: signing.azureTrustedSigning?.metadataPath
        ? `configured: ${signing.azureTrustedSigning.metadataPath}`
        : 'missing `VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH` or `package.signing.windows.azureTrustedSigning.metadataPath`',
    });
    return checks;
  }

  if (signing.provider === 'digicertKeyLocker') {
    const smctlTool = signing.digicertKeyLocker?.smctlPath ?? 'smctl';
    checks.push(
      createToolCheck(
        'signing.win.digicert.smctl',
        `DigiCert KeyLocker tool (\`${smctlTool}\`)`,
        smctlTool,
        deps.isToolAvailable,
        'required for DigiCert KeyLocker signing flow',
      ),
    );
    checks.push({
      id: 'signing.win.digicert.keypair',
      status: signing.digicertKeyLocker?.keypairAlias ? 'pass' : 'fail',
      title: 'DigiCert KeyLocker keypair alias',
      details: signing.digicertKeyLocker?.keypairAlias
        ? `configured: ${signing.digicertKeyLocker.keypairAlias}`
        : 'missing `VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS` or config value',
    });
    return checks;
  }

  const hasSignTool = deps.isToolAvailable('signtool') || deps.isToolAvailable('osslsigncode');
  checks.push({
    id: 'signing.win.local.tool',
    status: hasSignTool ? 'pass' : 'fail',
    title: 'Windows local signing tool (`signtool` or `osslsigncode`)',
    details: hasSignTool ? 'local signing tool detected' : 'install `signtool` or `osslsigncode`',
  });
  checks.push({
    id: 'signing.win.local.certificate',
    status: signing.certificate ? 'pass' : 'fail',
    title: 'Windows signing certificate',
    details: signing.certificate
      ? `configured: ${signing.certificate}`
      : 'missing `VOLT_WIN_CERTIFICATE` or `package.signing.windows.certificate`',
  });

  return checks;
}

function collectMacSigningChecks(
  packageConfig: PackageConfig,
  deps: DoctorDeps,
): DoctorCheckResult[] {
  const checks: DoctorCheckResult[] = [];
  const signing = resolveSigningConfig(packageConfig, 'darwin')?.macOS;
  if (!signing) {
    checks.push({
      id: 'signing.mac.disabled',
      status: 'warn',
      title: 'macOS signing configuration',
      details: 'not configured (set `package.signing.macOS` or related VOLT_MACOS_* env vars)',
    });
    return checks;
  }

  checks.push(
    createToolCheck(
      'signing.mac.codesign',
      'macOS signing tool (`codesign`)',
      'codesign',
      deps.isToolAvailable,
      'required for macOS signing',
    ),
  );
  checks.push({
    id: 'signing.mac.identity',
    status: signing.identity ? 'pass' : 'fail',
    title: 'macOS signing identity',
    details: signing.identity
      ? `configured: ${signing.identity}`
      : 'missing `VOLT_MACOS_SIGNING_IDENTITY` or `package.signing.macOS.identity`',
  });

  if (signing.notarize) {
    checks.push(
      createToolCheck(
        'signing.mac.notarytool',
        'macOS notarization tool (`xcrun`)',
        'xcrun',
        deps.isToolAvailable,
        'required for notarization (`xcrun notarytool`)',
      ),
    );
    checks.push({
      id: 'signing.mac.apple-id',
      status: deps.env['VOLT_APPLE_ID'] ? 'pass' : 'fail',
      title: 'Apple ID for notarization',
      details: deps.env['VOLT_APPLE_ID']
        ? 'configured via VOLT_APPLE_ID'
        : 'missing VOLT_APPLE_ID',
    });
    checks.push({
      id: 'signing.mac.apple-password',
      status: deps.env['VOLT_APPLE_PASSWORD'] ? 'pass' : 'fail',
      title: 'Apple app-specific password for notarization',
      details: deps.env['VOLT_APPLE_PASSWORD']
        ? 'configured via VOLT_APPLE_PASSWORD'
        : 'missing VOLT_APPLE_PASSWORD',
    });
    checks.push({
      id: 'signing.mac.team-id',
      status: signing.teamId ? 'pass' : 'fail',
      title: 'Apple team ID for notarization',
      details: signing.teamId
        ? `configured: ${signing.teamId}`
        : 'missing VOLT_APPLE_TEAM_ID or package.signing.macOS.teamId',
    });
  }

  return checks;
}

function createToolCheck(
  id: string,
  title: string,
  toolName: string,
  toolLookup: (toolName: string) => boolean,
  missingDetails: string,
): DoctorCheckResult {
  const available = toolLookup(toolName);
  return {
    id,
    status: available ? 'pass' : 'fail',
    title,
    details: available ? `${toolName} detected` : missingDetails,
  };
}

function printDoctorReport(report: DoctorReport): void {
  console.log(`[volt:doctor] Target platform: ${report.target}`);
  console.log(`[volt:doctor] Package formats: ${report.formats.join(', ')}`);
  for (const check of report.checks) {
    const tag = check.status.toUpperCase();
    console.log(`[volt:doctor] [${tag}] ${check.title}: ${check.details}`);
  }
  console.log(
    `[volt:doctor] Summary: pass=${report.summary.pass} warn=${report.summary.warn} fail=${report.summary.fail}`,
  );
}

export const __testOnly = {
  resolveDoctorFormats,
  collectDoctorChecks,
  summarizeDoctorChecks,
};
