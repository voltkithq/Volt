import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { devModuleError, ensureDevPermission, resolveProjectScopedPath } from './shared.js';

const storage = new Map<string, string>();
let loaded = false;
const MAX_VALUE_LENGTH = 8192;

function storageFilePath(): string {
  return resolveProjectScopedPath('storage.json', 'secure-storage');
}

function loadStorage(): void {
  if (loaded) {
    return;
  }
  loaded = true;

  const filePath = storageFilePath();
  if (!existsSync(filePath)) {
    return;
  }

  try {
    const raw = readFileSync(filePath, 'utf8');
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== 'object') {
      return;
    }
    for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof value === 'string') {
        storage.set(key, value);
      }
    }
  } catch {
    storage.clear();
  }
}

function persistStorage(): void {
  const jsonObject = Object.fromEntries(storage);
  writeFileSync(storageFilePath(), `${JSON.stringify(jsonObject, null, 2)}\n`, 'utf8');
}

function normalizeKey(key: string): string {
  const normalized = key.trim();
  if (!normalized) {
    throw devModuleError('secureStorage', 'Secure storage key must be a non-empty string.');
  }
  return normalized;
}

function normalizeValue(value: string): string {
  if (Buffer.byteLength(value, 'utf8') > MAX_VALUE_LENGTH) {
    throw devModuleError(
      'secureStorage',
      `Secure storage value length must be <= ${MAX_VALUE_LENGTH} bytes.`,
    );
  }
  return value;
}

export async function set(key: string, value: string): Promise<void> {
  ensureDevPermission('secureStorage', 'secureStorage.set()');
  loadStorage();
  storage.set(normalizeKey(key), normalizeValue(value));
  persistStorage();
}

export async function get(key: string): Promise<string | null> {
  ensureDevPermission('secureStorage', 'secureStorage.get()');
  loadStorage();
  const normalizedKey = normalizeKey(key);
  return storage.get(normalizedKey) ?? null;
}

async function remove(key: string): Promise<void> {
  ensureDevPermission('secureStorage', 'secureStorage.delete()');
  loadStorage();
  storage.delete(normalizeKey(key));
  persistStorage();
}

export { remove as delete };

export async function has(key: string): Promise<boolean> {
  ensureDevPermission('secureStorage', 'secureStorage.has()');
  loadStorage();
  return storage.has(normalizeKey(key));
}
