import { createHash } from 'node:crypto';
import { readFileSync, statSync } from 'node:fs';
import { basename } from 'node:path';
import type {
  PublishArtifactRecord,
  UpdateReleaseManifest,
} from './types.js';

const BASE64_ED25519_SIGNATURE_LENGTH = 64;

export function sha256FileHex(absolutePath: string): string {
  const bytes = readFileSync(absolutePath);
  return createHash('sha256').update(bytes).digest('hex');
}

export function buildPublishedArtifactRecord(
  absolutePath: string,
  baseUrl: string | undefined,
): PublishArtifactRecord {
  const fileName = basename(absolutePath);
  const sha256 = sha256FileHex(absolutePath);
  const size = statSync(absolutePath).size;
  const normalizedBaseUrl = normalizeBaseUrl(baseUrl);
  const url = normalizedBaseUrl ? `${normalizedBaseUrl}/${fileName}` : fileName;
  return { fileName, sha256, size, url };
}

export function buildUpdateReleaseManifest(args: {
  appName: string;
  channel: string;
  version: string;
  artifact: PublishArtifactRecord;
  signature: string;
}): UpdateReleaseManifest {
  const signature = args.signature.trim();
  if (!isValidMetadataSignature(signature)) {
    throw new Error(
      '[volt] Update manifest requires a base64 Ed25519 metadata signature.',
    );
  }

  return {
    schemaVersion: 1,
    appName: args.appName,
    channel: args.channel,
    generatedAt: new Date().toISOString(),
    update: {
      version: args.version,
      url: args.artifact.url,
      signature,
      sha256: args.artifact.sha256,
    },
    artifacts: [args.artifact],
  };
}

export function isValidMetadataSignature(value: string): boolean {
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(value) || value.length % 4 !== 0) {
    return false;
  }

  try {
    const decoded = Buffer.from(value, 'base64');
    return decoded.length === BASE64_ED25519_SIGNATURE_LENGTH
      && decoded.toString('base64') === value;
  } catch {
    return false;
  }
}

function normalizeBaseUrl(baseUrl: string | undefined): string | null {
  if (!baseUrl || baseUrl.trim().length === 0) {
    return null;
  }
  return baseUrl.replace(/\/+$/, '');
}
