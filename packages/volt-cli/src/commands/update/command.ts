import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { loadConfig } from '../../utils/config.js';
import { toSafeBinaryName } from '../../utils/naming.js';
import { resolveRuntimeArtifactForPackaging } from '../../utils/runtime-artifact.js';
import {
  buildPublishedArtifactRecord,
  buildUpdateReleaseManifest,
  isValidMetadataSignature,
} from './manifest.js';
import { createPublishProvider } from './provider.js';
import type { UpdatePublishOptions } from './types.js';

interface PreflightResult {
  errors: string[];
  artifactAbsolutePath: string | null;
  appName: string;
  version: string;
  signature: string | null;
}

export async function updatePublishCommand(options: UpdatePublishOptions): Promise<void> {
  const cwd = process.cwd();
  console.log('[volt] Preparing update publish flow...');

  const config = await loadConfig(cwd, { strict: true, commandName: 'update publish' });
  const artifactsDir = resolve(cwd, options.artifactsDir ?? 'dist-volt');
  const outDir = resolve(cwd, options.outDir ?? 'dist-update');
  const providerName = options.provider ?? 'local';
  const channel = (options.channel ?? 'stable').trim();
  const manifestFileName = (options.manifestFile ?? `manifest-${channel}.json`).trim();
  const dryRun = options.dryRun === true;

  const preflight = runPreflightChecks({
    config: config as unknown as Record<string, unknown>,
    artifactsDir,
  });
  if (preflight.errors.length > 0) {
    console.error('[volt] Update publish preflight failed:');
    for (const error of preflight.errors) {
      console.error(`  - ${error}`);
    }
    process.exit(1);
  }

  const artifactAbsolutePath = preflight.artifactAbsolutePath as string;
  const artifactRecord = buildPublishedArtifactRecord(artifactAbsolutePath, options.baseUrl);
  const manifest = buildUpdateReleaseManifest({
    appName: preflight.appName,
    channel,
    version: preflight.version,
    artifact: artifactRecord,
    signature: preflight.signature as string,
  });
  const manifestJson = `${JSON.stringify(manifest, null, 2)}\n`;

  const publishRoot = resolve(outDir, channel);
  const provider = createPublishProvider(providerName, publishRoot, dryRun);

  const artifactPublish = await provider.publishArtifact(artifactAbsolutePath, artifactRecord.fileName);
  const manifestPublish = await provider.publishManifest(manifestJson, manifestFileName);

  console.log(`[volt] Provider: ${provider.name}`);
  console.log(`[volt] Channel: ${channel}`);
  console.log(`[volt] Artifact: ${artifactRecord.fileName} (${artifactRecord.sha256})`);
  console.log(`[volt] Artifact published to: ${artifactPublish.location}`);
  console.log(`[volt] Manifest published to: ${manifestPublish.location}`);

  if (dryRun) {
    console.log('[volt] Dry run complete. No files were written.');
  } else {
    console.log('[volt] Update publish completed.');
  }
}

export function runPreflightChecks(args: {
  config: Record<string, unknown>;
  artifactsDir: string;
}): PreflightResult {
  const errors: string[] = [];
  const config = args.config;
  const appName = typeof config['name'] === 'string' ? config['name'] : 'Volt App';
  const rawVersion = typeof config['version'] === 'string' ? config['version'] : '0.1.0';
  const rawSignature = process.env['VOLT_UPDATE_SIGNATURE'];
  const signature = resolveUpdateSignatureFromEnv(rawSignature);

  if (!isLikelySemver(rawVersion)) {
    errors.push(`app version "${rawVersion}" is not a valid semver string`);
  }

  const updater = config['updater'] as Record<string, unknown> | undefined;
  if (!updater || typeof updater !== 'object') {
    errors.push('missing updater config; add updater.endpoint and updater.publicKey');
  } else {
    const endpoint = typeof updater['endpoint'] === 'string' ? updater['endpoint'].trim() : '';
    const publicKey = typeof updater['publicKey'] === 'string' ? updater['publicKey'].trim() : '';
    if (!endpoint) {
      errors.push('missing updater.endpoint in volt.config.ts');
    }
    if (!publicKey) {
      errors.push('missing updater.publicKey in volt.config.ts');
    }
  }

  if (!existsSync(args.artifactsDir)) {
    errors.push(`artifacts directory does not exist: ${args.artifactsDir}`);
    return {
      errors,
      artifactAbsolutePath: null,
      appName,
      version: rawVersion,
      signature,
    };
  }

  const binaryName = toSafeBinaryName(appName);
  const resolution = resolveRuntimeArtifactForPackaging(args.artifactsDir, binaryName);
  const artifact = resolution.artifact;
  if (!artifact) {
    errors.push(
      `no runtime artifact found in ${args.artifactsDir}; run \`volt build\` first`,
    );
  } else if (artifact.runtimeKind !== 'executable') {
    errors.push(
      `runtime artifact "${artifact.fileName}" is not executable; build must produce an executable update payload`,
    );
  }

  if (!rawSignature || rawSignature.trim().length === 0) {
    errors.push(
      'missing VOLT_UPDATE_SIGNATURE; set it to the base64 Ed25519 signature for the final update metadata payload before publishing',
    );
  } else if (!signature) {
    errors.push(
      'invalid VOLT_UPDATE_SIGNATURE; expected a base64 Ed25519 metadata signature',
    );
  }

  return {
    errors,
    artifactAbsolutePath: artifact?.absolutePath ?? null,
    appName,
    version: rawVersion,
    signature,
  };
}

function isLikelySemver(value: string): boolean {
  const trimmed = value.trim();
  return /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(trimmed);
}

function resolveUpdateSignatureFromEnv(raw: string | undefined): string | null {
  if (!raw) {
    return null;
  }

  const trimmed = raw.trim();
  if (!trimmed || !isValidMetadataSignature(trimmed)) {
    return null;
  }

  return trimmed;
}
