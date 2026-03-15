import { basename } from 'node:path';

import type { PackageArtifactSummary } from '../types.js';

export function generateDeploymentReadme(args: {
  appName: string;
  version: string;
  nsisInstaller: string | null;
  msixPackage: string | null;
}): string {
  return `# Enterprise Deployment Bundle

App: ${args.appName}
Version: ${args.version}

## Included Outputs

- ADMX policy: \`policy/Volt.admx\`
- ADML locale file: \`policy/en-US/Volt.adml\`
- Effective policy values: \`policy/policy-values.json\`
- Installation scripts: \`scripts/*.ps1\`

## Installer Artifacts

- NSIS installer: ${args.nsisInstaller ?? 'not generated in this packaging run'}
- MSIX package: ${args.msixPackage ?? 'not generated in this packaging run'}

## Group Policy Rollout

1. Copy \`policy/Volt.admx\` to your Central Store \`PolicyDefinitions\` directory.
2. Copy \`policy/en-US/Volt.adml\` to \`PolicyDefinitions/en-US\`.
3. Create or edit a GPO under:
   Computer Configuration -> Administrative Templates -> Volt Enterprise.
4. Apply policies, run \`gpupdate /force\`, and validate on a pilot machine.
`;
}

export function generateNsisInstallScript(installerName: string | null, allUsers: boolean): string {
  const scopeFlag = allUsers ? '/ALLUSERS=1' : '/ALLUSERS=0';
  const installerPath = installerName ?? '<installer-setup.exe>';
  return [
    "$ErrorActionPreference = 'Stop'",
    `$installer = Join-Path $PSScriptRoot '..\\..\\${installerPath}'`,
    'if (-not (Test-Path $installer)) {',
    '  throw "Installer not found: $installer"',
    '}',
    `Start-Process -FilePath $installer -ArgumentList '/S ${scopeFlag}' -Wait -NoNewWindow`,
  ].join('\n');
}

export function generateMsixInstallScript(msixName: string | null): string {
  const packagePath = msixName ?? '<app.msix>';
  return [
    "$ErrorActionPreference = 'Stop'",
    `$package = Join-Path $PSScriptRoot '..\\..\\${packagePath}'`,
    'if (-not (Test-Path $package)) {',
    '  throw "MSIX package not found: $package"',
    '}',
    'Add-AppxPackage -Path $package',
  ].join('\n');
}

export function firstArtifactBySuffix(
  artifacts: readonly PackageArtifactSummary[],
  suffix: string,
): string | null {
  const match = artifacts.find((artifact) =>
    artifact.fileName.toLowerCase().endsWith(suffix.toLowerCase()),
  );
  return match ? basename(match.path) : null;
}
