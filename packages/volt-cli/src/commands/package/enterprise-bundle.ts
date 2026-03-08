import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import type { VoltConfig } from 'voltkit';
import {
  ENTERPRISE_POLICY_SCHEMA,
  type EnterprisePolicyDefinition,
  type EnterprisePolicySchema,
} from './enterprise-schema.js';
import type { PackageArtifactSummary, PackageConfig, WindowsInstallMode } from './types.js';

export interface EnterpriseBundleOptions {
  appName: string;
  version: string;
  packageDir: string;
  packageConfig: PackageConfig;
  config: VoltConfig;
  installMode: WindowsInstallMode | null;
  artifacts: readonly PackageArtifactSummary[];
}

export interface EnterpriseBundleResult {
  bundleDir: string;
  generatedFiles: string[];
}

export function writeEnterpriseDeploymentBundle(options: EnterpriseBundleOptions): EnterpriseBundleResult {
  const shouldGenerateAdmx = options.packageConfig.enterprise?.generateAdmx !== false;
  const includeDocsBundle = options.packageConfig.enterprise?.includeDocsBundle !== false;

  if (!shouldGenerateAdmx && !includeDocsBundle) {
    return {
      bundleDir: resolve(options.packageDir, 'enterprise'),
      generatedFiles: [],
    };
  }

  const bundleDir = resolve(options.packageDir, 'enterprise');
  const policyDir = resolve(bundleDir, 'policy');
  const policyLocaleDir = resolve(policyDir, 'en-US');
  const scriptsDir = resolve(bundleDir, 'scripts');

  ensureDirectory(bundleDir, 'enterprise bundle');
  const generatedFiles: string[] = [];
  const nsisInstaller = firstArtifactBySuffix(options.artifacts, '-setup.exe');
  const msixPackage = firstArtifactBySuffix(options.artifacts, '.msix');

  if (shouldGenerateAdmx) {
    ensureDirectory(policyDir, 'enterprise policy');
    ensureDirectory(policyLocaleDir, 'enterprise policy locale');
    const admxPath = resolve(policyDir, 'Volt.admx');
    const admlPath = resolve(policyLocaleDir, 'Volt.adml');
    const policyValuesPath = resolve(policyDir, 'policy-values.json');

    safeWriteFile(admxPath, generateAdmx(ENTERPRISE_POLICY_SCHEMA));
    safeWriteFile(admlPath, generateAdml(ENTERPRISE_POLICY_SCHEMA));
    safeWriteFile(policyValuesPath, `${JSON.stringify(resolvePolicyValues(options), null, 2)}\n`);
    generatedFiles.push(admxPath, admlPath, policyValuesPath);
  }

  if (includeDocsBundle) {
    ensureDirectory(scriptsDir, 'enterprise scripts');
    const deploymentReadmePath = resolve(bundleDir, 'DEPLOYMENT.md');
    const installNsisMachinePath = resolve(scriptsDir, 'install-nsis-allusers.ps1');
    const installNsisUserPath = resolve(scriptsDir, 'install-nsis-current-user.ps1');
    const installMsixPath = resolve(scriptsDir, 'install-msix.ps1');

    safeWriteFile(
      deploymentReadmePath,
      generateDeploymentReadme({
        appName: options.appName,
        version: options.version,
        nsisInstaller,
        msixPackage,
      }),
    );
    safeWriteFile(installNsisMachinePath, generateNsisInstallScript(nsisInstaller, true));
    safeWriteFile(installNsisUserPath, generateNsisInstallScript(nsisInstaller, false));
    safeWriteFile(installMsixPath, generateMsixInstallScript(msixPackage));
    generatedFiles.push(
      deploymentReadmePath,
      installNsisMachinePath,
      installNsisUserPath,
      installMsixPath,
    );
  }

  return {
    bundleDir,
    generatedFiles,
  };
}

export function generateAdmx(schema: EnterprisePolicySchema): string {
  const categoryId = 'CategoryVoltEnterprise';
  const policyEntries = schema.policies.map((policy) => renderAdmxPolicy(policy)).join('\n');

  return `<?xml version="1.0" encoding="utf-8"?>
<policyDefinitions revision="1.0" schemaVersion="1.0">
  <policyNamespaces>
    <target namespace="${escapeXml(schema.namespace)}" prefix="volt" />
    <using namespace="Microsoft.Policies.Windows" prefix="windows" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories>
    <category name="${categoryId}" displayName="$(string.${categoryId})" />
  </categories>
  <policies>
${policyEntries}
  </policies>
</policyDefinitions>
`;
}

export function generateAdml(schema: EnterprisePolicySchema): string {
  const categoryId = 'CategoryVoltEnterprise';
  const stringRows: string[] = [
    `      <string id="${categoryId}">${escapeXml(schema.categoryDisplayName)}</string>`,
  ];
  const presentationRows: string[] = [];

  for (const policy of schema.policies) {
    stringRows.push(`      <string id="Policy_${policy.id}">${escapeXml(policy.displayName)}</string>`);
    stringRows.push(`      <string id="Policy_${policy.id}_Help">${escapeXml(policy.description)}</string>`);

    if (policy.type === 'enum') {
      for (const option of policy.enumValues ?? []) {
        stringRows.push(
          `      <string id="Policy_${policy.id}_${option.id}">${escapeXml(option.displayName)}</string>`,
        );
      }
    }

    const presentation = renderAdmlPresentation(policy);
    if (presentation) {
      presentationRows.push(presentation);
    }
  }

  return `<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <displayName>Volt Enterprise Policies</displayName>
  <description>Group Policy templates for Volt-managed desktop deployments.</description>
  <resources>
    <stringTable>
${stringRows.join('\n')}
    </stringTable>
    <presentationTable>
${presentationRows.join('\n')}
    </presentationTable>
  </resources>
</policyDefinitionResources>
`;
}

function renderAdmxPolicy(policy: EnterprisePolicyDefinition): string {
  const escapedId = escapeXml(policy.id);
  const escapedKey = escapeXml(policy.registryKey);
  const escapedValueName = escapeXml(policy.valueName);
  const presentation = policy.type === 'boolean' ? '' : ` presentation="$(presentation.Pres_${escapedId})"`;

  const body = renderAdmxPolicyBody(policy);
  return `    <policy name="${escapedId}" class="Machine" displayName="$(string.Policy_${escapedId})" explainText="$(string.Policy_${escapedId}_Help)" key="${escapedKey}" valueName="${escapedValueName}"${presentation}>
${body}
      <parentCategory ref="CategoryVoltEnterprise" />
    </policy>`;
}

function renderAdmxPolicyBody(policy: EnterprisePolicyDefinition): string {
  if (policy.type === 'boolean') {
    return [
      '      <enabledValue><decimal value="1" /></enabledValue>',
      '      <disabledValue><decimal value="0" /></disabledValue>',
    ].join('\n');
  }

  if (policy.type === 'text') {
    return [
      '      <elements>',
      `        <text id="${escapeXml(policy.id)}" valueName="${escapeXml(policy.valueName)}" required="true" />`,
      '      </elements>',
    ].join('\n');
  }

  if (policy.type === 'decimal') {
    const minValue = Number.isFinite(policy.minValue) ? policy.minValue : 1;
    const maxValue = Number.isFinite(policy.maxValue) ? policy.maxValue : 9999;
    return [
      '      <elements>',
      `        <decimal id="${escapeXml(policy.id)}" valueName="${escapeXml(policy.valueName)}" minValue="${minValue}" maxValue="${maxValue}" />`,
      '      </elements>',
    ].join('\n');
  }

  const enumItems = (policy.enumValues ?? [])
    .map((option) =>
      [
        `          <item displayName="$(string.Policy_${escapeXml(policy.id)}_${escapeXml(option.id)})">`,
        `            <value><string>${escapeXml(option.value)}</string></value>`,
        '          </item>',
      ].join('\n'))
    .join('\n');

  return [
    '      <elements>',
    `        <enum id="${escapeXml(policy.id)}" valueName="${escapeXml(policy.valueName)}">`,
    enumItems,
    '        </enum>',
    '      </elements>',
  ].join('\n');
}

function renderAdmlPresentation(policy: EnterprisePolicyDefinition): string | null {
  const presentationId = `Pres_${policy.id}`;
  if (policy.type === 'text') {
    const defaultValue = typeof policy.defaultValue === 'string' ? escapeXml(policy.defaultValue) : '';
    return [
      `      <presentation id="${escapeXml(presentationId)}">`,
      `        <textBox refId="${escapeXml(policy.id)}" defaultValue="${defaultValue}" />`,
      '      </presentation>',
    ].join('\n');
  }

  if (policy.type === 'decimal') {
    const defaultValue = typeof policy.defaultValue === 'number' ? policy.defaultValue : 0;
    return [
      `      <presentation id="${escapeXml(presentationId)}">`,
      `        <decimalTextBox refId="${escapeXml(policy.id)}" defaultValue="${defaultValue}" spin="1" />`,
      '      </presentation>',
    ].join('\n');
  }

  if (policy.type === 'enum') {
    return [
      `      <presentation id="${escapeXml(presentationId)}">`,
      `        <dropdownList refId="${escapeXml(policy.id)}" />`,
      '      </presentation>',
    ].join('\n');
  }

  return null;
}

function resolvePolicyValues(options: EnterpriseBundleOptions): Record<string, unknown> {
  const values: Record<string, unknown> = {};
  for (const policy of ENTERPRISE_POLICY_SCHEMA.policies) {
    const fromConfig = policy.id === 'InstallMode'
      ? options.installMode
      : readValueAtPath(options.config as unknown as Record<string, unknown>, policy.configPath);
    values[policy.id] = fromConfig ?? policy.defaultValue ?? null;
  }
  return values;
}

function readValueAtPath(value: Record<string, unknown>, path: string): unknown {
  const segments = path.split('.');
  let current: unknown = value;
  for (const segment of segments) {
    if (!current || typeof current !== 'object') {
      return undefined;
    }
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}

function generateDeploymentReadme(args: {
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

function generateNsisInstallScript(installerName: string | null, allUsers: boolean): string {
  const scopeFlag = allUsers ? '/ALLUSERS=1' : '/ALLUSERS=0';
  const installerPath = installerName ?? '<installer-setup.exe>';
  return [
    '$ErrorActionPreference = \'Stop\'',
    `$installer = Join-Path $PSScriptRoot '..\\..\\${installerPath}'`,
    'if (-not (Test-Path $installer)) {',
    '  throw "Installer not found: $installer"',
    '}',
    `Start-Process -FilePath $installer -ArgumentList '/S ${scopeFlag}' -Wait -NoNewWindow`,
  ].join('\n');
}

function generateMsixInstallScript(msixName: string | null): string {
  const packagePath = msixName ?? '<app.msix>';
  return [
    '$ErrorActionPreference = \'Stop\'',
    `$package = Join-Path $PSScriptRoot '..\\..\\${packagePath}'`,
    'if (-not (Test-Path $package)) {',
    '  throw "MSIX package not found: $package"',
    '}',
    'Add-AppxPackage -Path $package',
  ].join('\n');
}

function firstArtifactBySuffix(artifacts: readonly PackageArtifactSummary[], suffix: string): string | null {
  const match = artifacts.find((artifact) => artifact.fileName.toLowerCase().endsWith(suffix.toLowerCase()));
  return match ? basename(match.path) : null;
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function ensureDirectory(path: string, label: string): void {
  try {
    mkdirSync(path, { recursive: true });
  } catch (error) {
    throw new Error(
      `[volt] Failed to create ${label} directory at ${path}: ${toErrorMessage(error)}`,
    );
  }
}

function safeWriteFile(path: string, contents: string): void {
  try {
    writeFileSync(path, contents, 'utf8');
  } catch (error) {
    throw new Error(`[volt] Failed to write enterprise bundle file ${path}: ${toErrorMessage(error)}`);
  }
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
