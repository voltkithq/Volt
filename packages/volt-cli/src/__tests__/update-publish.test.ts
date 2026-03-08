import { mkdtempSync, existsSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/update.js';

function createTempDir(prefix: string): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

describe('update publish scaffolding', () => {
  const validSignature = Buffer.alloc(64, 9).toString('base64');
  const originalSignature = process.env['VOLT_UPDATE_SIGNATURE'];

  beforeEach(() => {
    process.env['VOLT_UPDATE_SIGNATURE'] = validSignature;
  });

  afterEach(() => {
    if (originalSignature === undefined) {
      delete process.env['VOLT_UPDATE_SIGNATURE'];
    } else {
      process.env['VOLT_UPDATE_SIGNATURE'] = originalSignature;
    }
  });

  it('fails preflight when updater config is missing', () => {
    const artifactsDir = createTempDir('volt-update-publish-missing-updater-');
    const result = __testOnly.runPreflightChecks({
      config: { name: 'Volt App', version: '1.0.0' },
      artifactsDir,
    });
    expect(result.errors.some((error) => error.includes('missing updater config'))).toBe(true);
  });

  it('passes preflight when executable artifact is present', () => {
    const artifactsDir = createTempDir('volt-update-publish-ok-');
    writeFileSync(join(artifactsDir, 'volt-app.exe'), 'binary');
    const result = __testOnly.runPreflightChecks({
      config: {
        name: 'Volt App',
        version: '1.0.0',
        updater: {
          endpoint: 'https://updates.example.com/check',
          publicKey: 'test-public-key',
        },
      },
      artifactsDir,
    });
    expect(result.errors).toEqual([]);
    expect(result.artifactAbsolutePath).toBe(resolve(artifactsDir, 'volt-app.exe'));
    expect(result.signature).toBe(validSignature);
  });

  it('builds deterministic artifact metadata + manifest shape', () => {
    const artifactsDir = createTempDir('volt-update-publish-manifest-');
    const artifactPath = join(artifactsDir, 'volt-app.exe');
    writeFileSync(artifactPath, 'demo-bytes');
    const artifact = __testOnly.buildPublishedArtifactRecord(
      artifactPath,
      'https://updates.example.com/releases',
    );
    const manifest = __testOnly.buildUpdateReleaseManifest({
      appName: 'Volt App',
      channel: 'stable',
      version: '1.2.3',
      artifact,
      signature: validSignature,
    });

    expect(manifest.schemaVersion).toBe(1);
    expect(manifest.update.version).toBe('1.2.3');
    expect(manifest.update.url).toBe('https://updates.example.com/releases/volt-app.exe');
    expect(manifest.update.signature).toBe(validSignature);
    expect(manifest.update.sha256).toBe(artifact.sha256);
    expect(manifest.artifacts).toHaveLength(1);
  });

  it('fails preflight when update signature is malformed', () => {
    process.env['VOLT_UPDATE_SIGNATURE'] = 'dGVzdC1zaWduYXR1cmU=';
    const artifactsDir = createTempDir('volt-update-publish-bad-signature-');
    writeFileSync(join(artifactsDir, 'volt-app.exe'), 'binary');

    const result = __testOnly.runPreflightChecks({
      config: {
        name: 'Volt App',
        version: '1.0.0',
        updater: {
          endpoint: 'https://updates.example.com/check',
          publicKey: 'test-public-key',
        },
      },
      artifactsDir,
    });

    expect(
      result.errors.some((error) => error.includes('invalid VOLT_UPDATE_SIGNATURE')),
    ).toBe(true);
  });

  it('fails preflight when update signature is missing', () => {
    delete process.env['VOLT_UPDATE_SIGNATURE'];
    const artifactsDir = createTempDir('volt-update-publish-missing-signature-');
    writeFileSync(join(artifactsDir, 'volt-app.exe'), 'binary');

    const result = __testOnly.runPreflightChecks({
      config: {
        name: 'Volt App',
        version: '1.0.0',
        updater: {
          endpoint: 'https://updates.example.com/check',
          publicKey: 'test-public-key',
        },
      },
      artifactsDir,
    });

    expect(
      result.errors.some((error) => error.includes('missing VOLT_UPDATE_SIGNATURE')),
    ).toBe(true);
  });

  it('uses no-op behavior for local provider dry-run mode', async () => {
    const publishDir = createTempDir('volt-update-publish-dryrun-');
    const provider = __testOnly.createPublishProvider('local', publishDir, true);
    const result = await provider.publishManifest('{"ok":true}\n', 'manifest.json');
    expect(existsSync(result.location)).toBe(false);
  });

  it('writes files when local provider is not in dry-run mode', async () => {
    const publishDir = createTempDir('volt-update-publish-write-');
    const sourceDir = createTempDir('volt-update-publish-write-src-');
    const sourceArtifact = join(sourceDir, 'volt-app.exe');
    writeFileSync(sourceArtifact, 'bytes');

    const provider = __testOnly.createPublishProvider('local', publishDir, false);
    const artifactResult = await provider.publishArtifact(sourceArtifact, 'volt-app.exe');
    const manifestResult = await provider.publishManifest('{"ok":true}\n', 'manifest.json');

    expect(existsSync(artifactResult.location)).toBe(true);
    expect(existsSync(manifestResult.location)).toBe(true);
    expect(readFileSync(manifestResult.location, 'utf8')).toContain('"ok":true');
  });
});
