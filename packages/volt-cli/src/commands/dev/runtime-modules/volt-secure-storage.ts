import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolveProjectScopedPath } from './shared.js';

const storage = new Map<string, string>();
let loaded = false;

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
    throw new Error('Secure storage key must be a non-empty string.');
  }
  return normalized;
}

export async function set(key: string, value: string): Promise<void> {
  loadStorage();
  storage.set(normalizeKey(key), value);
  persistStorage();
}

export async function get(key: string): Promise<string | null> {
  loadStorage();
  const normalizedKey = normalizeKey(key);
  return storage.get(normalizedKey) ?? null;
}

async function remove(key: string): Promise<void> {
  loadStorage();
  storage.delete(normalizeKey(key));
  persistStorage();
}

export { remove as delete };

export async function has(key: string): Promise<boolean> {
  loadStorage();
  return storage.has(normalizeKey(key));
}

