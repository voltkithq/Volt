import type { WindowsSigningProvider } from './types.js';

const WINDOWS_PROVIDER_ALIASES: Readonly<Record<string, WindowsSigningProvider>> = {
  azure: 'azureTrustedSigning',
  azure_trusted_signing: 'azureTrustedSigning',
  azuretrustedsigning: 'azureTrustedSigning',
  digicert: 'digicertKeyLocker',
  digicert_keylocker: 'digicertKeyLocker',
  digicertkeylocker: 'digicertKeyLocker',
  keylocker: 'digicertKeyLocker',
  local: 'local',
};

/**
 * Normalize provider aliases into the canonical Windows signing provider value.
 * Returns null when the input is empty or unsupported.
 */
export function normalizeWindowsSigningProvider(raw: string | undefined): WindowsSigningProvider | null {
  if (typeof raw !== 'string') {
    return null;
  }

  const normalized = raw.trim().toLowerCase();
  if (normalized.length === 0) {
    return null;
  }

  return WINDOWS_PROVIDER_ALIASES[normalized] ?? null;
}

/**
 * Resolve provider aliases and fall back to a default when the input is unset or unsupported.
 */
export function resolveWindowsSigningProvider(
  raw: string | undefined,
  fallback: WindowsSigningProvider = 'local',
): WindowsSigningProvider {
  return normalizeWindowsSigningProvider(raw) ?? fallback;
}
