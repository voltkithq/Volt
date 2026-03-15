import { resolveSigningConfig } from '../../utils/signing.js';

import type { DoctorCheckContext, DoctorCheckResult, DoctorDeps } from './types.js';

export function createToolCheck(
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

export function collectWindowsSigningChecks(
  context: DoctorCheckContext,
  deps: DoctorDeps,
): DoctorCheckResult[] {
  const checks: DoctorCheckResult[] = [];
  const signing = resolveSigningConfig(context.packageConfig, 'win32')?.windows;
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

export function collectMacSigningChecks(
  context: DoctorCheckContext,
  deps: DoctorDeps,
): DoctorCheckResult[] {
  const checks: DoctorCheckResult[] = [];
  const signing = resolveSigningConfig(context.packageConfig, 'darwin')?.macOS;
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
      details: deps.env['VOLT_APPLE_ID'] ? 'configured via VOLT_APPLE_ID' : 'missing VOLT_APPLE_ID',
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
