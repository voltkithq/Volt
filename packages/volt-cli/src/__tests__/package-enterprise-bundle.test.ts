import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import type { VoltConfig } from 'voltkit';
import { writeEnterpriseDeploymentBundle } from '../commands/package/enterprise-bundle.js';

const tempDirs: string[] = [];

function createTempDir(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    if (!dir) {
      continue;
    }
    rmSync(dir, { recursive: true, force: true });
  }
});

describe('enterprise bundle generation', () => {
  it('writes ADMX/ADML, policy values, docs, and install scripts by default', () => {
    const packageDir = createTempDir('volt-enterprise-package-');
    const config: VoltConfig = {
      name: 'Volt App',
      version: '1.0.0',
      devtools: false,
      runtime: { poolSize: 4 },
      updater: {
        endpoint: 'https://updates.example.com/check',
        publicKey: 'example-key',
      },
      package: {
        identifier: 'com.example.voltapp',
      },
    };

    const result = writeEnterpriseDeploymentBundle({
      appName: 'Volt App',
      version: '1.0.0',
      packageDir,
      packageConfig: config.package!,
      config,
      installMode: 'perMachine',
      artifacts: [
        { path: resolve(packageDir, 'volt-app-1.0.0-setup.exe'), fileName: 'volt-app-1.0.0-setup.exe' },
        { path: resolve(packageDir, 'volt-app-1.0.0.msix'), fileName: 'volt-app-1.0.0.msix' },
      ],
    });

    expect(result.generatedFiles).toHaveLength(7);
    expect(existsSync(resolve(packageDir, 'enterprise', 'policy', 'Volt.admx'))).toBe(true);
    expect(existsSync(resolve(packageDir, 'enterprise', 'policy', 'en-US', 'Volt.adml'))).toBe(true);
    expect(existsSync(resolve(packageDir, 'enterprise', 'DEPLOYMENT.md'))).toBe(true);

    const policyValues = JSON.parse(
      readFileSync(resolve(packageDir, 'enterprise', 'policy', 'policy-values.json'), 'utf8'),
    ) as Record<string, unknown>;
    expect(policyValues['InstallMode']).toBe('perMachine');
    expect(policyValues['EnableDevtools']).toBe(false);
  });

  it('supports ADMX-only mode when docs bundle is disabled', () => {
    const packageDir = createTempDir('volt-enterprise-policy-only-');
    const config: VoltConfig = {
      name: 'Volt App',
      package: {
        identifier: 'com.example.voltapp',
        enterprise: {
          generateAdmx: true,
          includeDocsBundle: false,
        },
      },
    };

    const result = writeEnterpriseDeploymentBundle({
      appName: 'Volt App',
      version: '1.0.0',
      packageDir,
      packageConfig: config.package!,
      config,
      installMode: 'perUser',
      artifacts: [],
    });

    expect(result.generatedFiles).toHaveLength(3);
    expect(existsSync(resolve(packageDir, 'enterprise', 'policy', 'Volt.admx'))).toBe(true);
    expect(existsSync(resolve(packageDir, 'enterprise', 'DEPLOYMENT.md'))).toBe(false);
  });

  it('skips bundle generation when both enterprise outputs are disabled', () => {
    const packageDir = createTempDir('volt-enterprise-disabled-');
    const config: VoltConfig = {
      name: 'Volt App',
      package: {
        identifier: 'com.example.voltapp',
        enterprise: {
          generateAdmx: false,
          includeDocsBundle: false,
        },
      },
    };

    const result = writeEnterpriseDeploymentBundle({
      appName: 'Volt App',
      version: '1.0.0',
      packageDir,
      packageConfig: config.package!,
      config,
      installMode: null,
      artifacts: [],
    });

    expect(result.generatedFiles).toHaveLength(0);
    expect(existsSync(resolve(packageDir, 'enterprise'))).toBe(false);
  });

  it('throws a clear error when enterprise bundle directory creation fails', () => {
    const rootDir = createTempDir('volt-enterprise-io-error-');
    const blockingPath = resolve(rootDir, 'blocked-file');
    writeFileSync(blockingPath, 'blocked', 'utf8');

    const config: VoltConfig = {
      name: 'Volt App',
      package: {
        identifier: 'com.example.voltapp',
      },
    };

    expect(() =>
      writeEnterpriseDeploymentBundle({
        appName: 'Volt App',
        version: '1.0.0',
        packageDir: blockingPath,
        packageConfig: config.package!,
        config,
        installMode: 'perMachine',
        artifacts: [],
      }),
    ).toThrow('Failed to create enterprise bundle directory');
  });
});
