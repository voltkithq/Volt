import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

const STORAGE_DIR = 'storage';
const STORAGE_INDEX = '_index.json';

interface StorageIndex {
  entries: Record<string, string>;
}

export function performStorageRequest(
  dataRoot: string,
  operation: string,
  payload: Record<string, unknown> | null,
): unknown {
  const storageRoot = join(dataRoot, STORAGE_DIR);
  mkdirSync(storageRoot, { recursive: true });
  const index = loadIndex(storageRoot);

  switch (operation) {
    case 'get': {
      const key = requireKey(payload);
      const hash = index.entries[key];
      return hash ? readValueIfPresent(storageRoot, hash) : null;
    }
    case 'set': {
      const key = requireKey(payload);
      const hash = hashKey(key);
      writeFileSync(join(storageRoot, `${hash}.val`), requireValue(payload), 'utf8');
      index.entries[key] = hash;
      saveIndex(storageRoot, index);
      return null;
    }
    case 'has':
      return performStorageRequest(dataRoot, 'get', payload) !== null;
    case 'delete': {
      const key = requireKey(payload);
      const hash = index.entries[key];
      delete index.entries[key];
      if (hash) {
        rmSync(join(storageRoot, `${hash}.val`), { force: true });
      }
      saveIndex(storageRoot, index);
      return null;
    }
    case 'keys':
      reconcileIndex(storageRoot, index);
      return Object.keys(index.entries);
    default:
      throw new Error(`unsupported storage operation '${operation}'`);
  }
}

function loadIndex(storageRoot: string): StorageIndex {
  const indexPath = join(storageRoot, STORAGE_INDEX);
  if (!existsSync(indexPath)) {
    return { entries: {} };
  }
  return JSON.parse(readFileSync(indexPath, 'utf8')) as StorageIndex;
}

function saveIndex(storageRoot: string, index: StorageIndex): void {
  writeFileSync(join(storageRoot, STORAGE_INDEX), `${JSON.stringify(index, null, 2)}\n`, 'utf8');
}

function reconcileIndex(storageRoot: string, index: StorageIndex): void {
  for (const [key, hash] of Object.entries(index.entries)) {
    if (!existsSync(join(storageRoot, `${hash}.val`))) {
      delete index.entries[key];
    }
  }
  for (const name of readdirSync(storageRoot)) {
    if (!name.endsWith('.val')) {
      continue;
    }
    const tracked = Object.values(index.entries).some((hash) => `${hash}.val` === name);
    if (!tracked) {
      rmSync(join(storageRoot, name), { force: true });
    }
  }
  saveIndex(storageRoot, index);
}

function readValueIfPresent(storageRoot: string, hash: string): string | null {
  const path = join(storageRoot, `${hash}.val`);
  return existsSync(path) ? readFileSync(path, 'utf8') : null;
}

function hashKey(key: string): string {
  return createHash('sha256').update(key).digest('hex');
}

function requireKey(payload: Record<string, unknown> | null): string {
  const key = payload?.['key'];
  if (typeof key !== 'string' || key.trim().length === 0) {
    throw new Error("payload is missing required 'key' string");
  }
  if (key.length > 256 || key.includes('..') || key.includes('/') || key.includes('\\')) {
    throw new Error('invalid storage key');
  }
  return key;
}

function requireValue(payload: Record<string, unknown> | null): string {
  const value = payload?.['value'];
  if (typeof value !== 'string') {
    throw new Error("payload is missing required 'value' string");
  }
  if (value.length > 1024 * 1024) {
    throw new Error('storage value exceeds 1048576 bytes');
  }
  return value;
}
