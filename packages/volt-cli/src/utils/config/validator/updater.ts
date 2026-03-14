import type { VoltConfig } from 'voltkit';
import { pushError, type ValidationContext } from './context.js';

const BASE64_ED25519_PUBLIC_KEY_LENGTH = 32;
const LOOPBACK_IPV4_OCTET_COUNT = 4;

export function validateUpdaterConfig(config: VoltConfig, context: ValidationContext): void {
  if (!config.updater) {
    return;
  }

  const updater = config.updater;
  let updaterValid = true;
  if (!updater.endpoint || typeof updater.endpoint !== 'string' || !isValidUpdaterEndpoint(updater.endpoint)) {
    pushError(
      context,
      `'updater.endpoint' must be an HTTPS URL or an HTTP localhost/loopback URL for local testing.`,
    );
    updaterValid = false;
  }
  if (!updater.publicKey || typeof updater.publicKey !== 'string' || !isValidEd25519PublicKey(updater.publicKey)) {
    pushError(context, `'updater.publicKey' must be a base64 Ed25519 public key.`);
    updaterValid = false;
  }
  if (!updaterValid) {
    config.updater = undefined;
  }
}

function isValidUpdaterEndpoint(value: string): boolean {
  try {
    const parsed = new URL(value.trim());
    if (parsed.protocol === 'https:') {
      return true;
    }
    if (parsed.protocol !== 'http:') {
      return false;
    }

    const hostname = parsed.hostname.toLowerCase().replace(/^\[|\]$/g, '');
    return hostname === 'localhost' || hostname === '::1' || isLoopbackIpv4(hostname);
  } catch {
    return false;
  }
}

function isLoopbackIpv4(hostname: string): boolean {
  const octets = hostname.split('.');
  if (octets.length !== LOOPBACK_IPV4_OCTET_COUNT) {
    return false;
  }

  const numbers = octets.map((segment) => Number.parseInt(segment, 10));
  if (numbers.some((value) => Number.isNaN(value) || value < 0 || value > 255)) {
    return false;
  }

  return numbers[0] === 127;
}

function isValidEd25519PublicKey(value: string): boolean {
  const trimmed = value.trim();
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(trimmed) || trimmed.length % 4 !== 0) {
    return false;
  }

  try {
    const decoded = Buffer.from(trimmed, 'base64');
    return decoded.length === BASE64_ED25519_PUBLIC_KEY_LENGTH
      && decoded.toString('base64') === trimmed;
  } catch {
    return false;
  }
}
