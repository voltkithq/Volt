import { resolve } from 'node:path';

import { ENTERPRISE_POLICY_SCHEMA } from '../enterprise-schema.js';
import {
  firstArtifactBySuffix,
  generateDeploymentReadme,
  generateMsixInstallScript,
  generateNsisInstallScript,
} from './bundle-templates.js';
import { ensureDirectory, safeWriteFile } from './fs.js';
import { resolvePolicyValues } from './policy-values.js';
import { generateAdml, generateAdmx } from './policy-templates.js';
import type { EnterpriseBundleOptions, EnterpriseBundleResult } from './types.js';

export function writeEnterpriseDeploymentBundle(
  options: EnterpriseBundleOptions,
): EnterpriseBundleResult {
  const shouldGenerateAdmx = options.packageConfig.enterprise?.generateAdmx !== false;
  const includeDocsBundle = options.packageConfig.enterprise?.includeDocsBundle !== false;
  const bundleDir = resolve(options.packageDir, 'enterprise');

  if (!shouldGenerateAdmx && !includeDocsBundle) {
    return { bundleDir, generatedFiles: [] };
  }

  const policyDir = resolve(bundleDir, 'policy');
  const policyLocaleDir = resolve(policyDir, 'en-US');
  const scriptsDir = resolve(bundleDir, 'scripts');
  const generatedFiles: string[] = [];
  const nsisInstaller = firstArtifactBySuffix(options.artifacts, '-setup.exe');
  const msixPackage = firstArtifactBySuffix(options.artifacts, '.msix');

  ensureDirectory(bundleDir, 'enterprise bundle');

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
