import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { signSetupCommand, signTestOnly } from '../commands/sign.js';

const tempDirs: string[] = [];
const originalCwd = process.cwd();

function createTempProject(configBody: string): string {
  const dir = mkdtempSync(join(tmpdir(), 'volt-sign-setup-'));
  tempDirs.push(dir);
  writeFileSync(join(dir, 'volt.config.mjs'), configBody, 'utf8');
  return dir;
}

afterEach(() => {
  process.chdir(originalCwd);
  vi.restoreAllMocks();

  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    if (!dir) {
      continue;
    }
    rmSync(dir, { recursive: true, force: true });
  }
});

describe('sign setup command', () => {
  it('builds provider-specific Windows template sections', () => {
    const azureTemplate = signTestOnly.buildSigningEnvTemplate({
      platform: 'win32',
      windowsProvider: 'azureTrustedSigning',
    });
    expect(azureTemplate).toContain('VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH=');
    expect(azureTemplate).toContain('VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH=');

    const digicertTemplate = signTestOnly.buildSigningEnvTemplate({
      platform: 'win32',
      windowsProvider: 'digicertKeyLocker',
    });
    expect(digicertTemplate).toContain('VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS=');
    expect(digicertTemplate).toContain('VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH=smctl');
  });

  it('writes signing bootstrap template to output path', async () => {
    const projectDir = createTempProject(
      `export default {
  name: 'My App',
  package: {
    identifier: 'com.example.my-app',
    signing: {
      windows: { provider: 'azureTrustedSigning' }
    }
  }
};\n`,
    );
    process.chdir(projectDir);

    await signSetupCommand({ output: '.env.signing.ci' });

    const outputPath = join(projectDir, '.env.signing.ci');
    expect(existsSync(outputPath)).toBe(true);
    const content = readFileSync(outputPath, 'utf8');
    expect(content).toContain('VOLT_WIN_SIGNING_PROVIDER=azureTrustedSigning');
    expect(content).toContain('VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH=');
  });

  it('refuses to overwrite existing template without --force', async () => {
    const projectDir = createTempProject(
      `export default {
  name: 'My App',
  package: { identifier: 'com.example.my-app' }
};\n`,
    );
    process.chdir(projectDir);

    const outputPath = join(projectDir, '.env.signing');
    writeFileSync(outputPath, 'existing=1\n', 'utf8');

    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      const normalized = code === undefined || code === null ? 0 : Number(code);
      throw new Error(`__PROCESS_EXIT__${Number.isNaN(normalized) ? 1 : normalized}`);
    }) as never);

    await expect(signSetupCommand({ output: '.env.signing' })).rejects.toThrow('__PROCESS_EXIT__1');
    expect(readFileSync(outputPath, 'utf8')).toBe('existing=1\n');

    exitSpy.mockRestore();
  });

  it('supports print-only mode without writing a file', async () => {
    const projectDir = createTempProject(
      `export default {
  name: 'My App',
  package: { identifier: 'com.example.my-app' }
};\n`,
    );
    process.chdir(projectDir);

    await signSetupCommand({ output: '.env.print-only', printOnly: true, platform: 'all' });
    expect(existsSync(join(projectDir, '.env.print-only'))).toBe(false);
  });
});
