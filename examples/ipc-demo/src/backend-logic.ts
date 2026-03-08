export interface ComputeArgs {
  a: number;
  b: number;
}

export interface DbRecord {
  id: string;
  message: string;
  createdAt: number;
}

export interface NativeIntegrationState {
  menuConfigured: boolean;
  shortcutRegistered: boolean;
  trayReady: boolean;
}

export interface ClipboardStatus {
  read: string;
  hasText: boolean;
}

const MAX_SECRET_KEY_LENGTH = 128;
const MAX_SECRET_VALUE_LENGTH = 4096;

export function toFiniteNumber(value: unknown, field: string): number {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  throw new Error(`compute.${field} must be a finite number`);
}

export function formatUuidFromHash(hash: string): string {
  const hex = hash.length >= 32 ? hash.slice(0, 32) : hash.padEnd(32, '0');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
}

export function toDbRecords(rows: unknown): DbRecord[] {
  if (!Array.isArray(rows)) {
    return [];
  }

  return rows
    .map((row) => {
      const value = row as Record<string, unknown>;
      if (typeof value.id !== 'string' || typeof value.message !== 'string') {
        return null;
      }
      const createdAtRaw = value.created_at;
      const createdAt = typeof createdAtRaw === 'number' ? createdAtRaw : Number(createdAtRaw ?? 0);
      return {
        id: value.id,
        message: value.message,
        createdAt: Number.isFinite(createdAt) ? createdAt : 0,
      } satisfies DbRecord;
    })
    .filter((row): row is DbRecord => row !== null);
}

export function extractDbRowsCount(row: unknown): number {
  if (!row || typeof row !== 'object') {
    return 0;
  }
  const total = (row as Record<string, unknown>).total;
  if (typeof total === 'number' && Number.isFinite(total)) {
    return total;
  }
  const parsed = Number(total);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function summarizeClipboardRead(value: unknown): ClipboardStatus {
  const read = typeof value === 'string' ? value : '';
  return {
    read,
    hasText: read.length > 0,
  };
}

export function evaluateNativeReady(state: NativeIntegrationState): boolean {
  return state.menuConfigured && state.shortcutRegistered && state.trayReady;
}

export function normalizeSecretKey(value: unknown): string {
  if (typeof value !== 'string') {
    throw new Error('secureStorage.key must be a string');
  }

  const key = value.trim();
  if (!key) {
    throw new Error('secureStorage.key must not be empty');
  }
  if (key.length > MAX_SECRET_KEY_LENGTH) {
    throw new Error(`secureStorage.key must be <= ${MAX_SECRET_KEY_LENGTH} characters`);
  }

  return key;
}

export function normalizeSecretValue(value: unknown): string {
  if (typeof value !== 'string') {
    throw new Error('secureStorage.value must be a string');
  }
  if (!value.trim()) {
    throw new Error('secureStorage.value must not be empty');
  }
  if (value.length > MAX_SECRET_VALUE_LENGTH) {
    throw new Error(`secureStorage.value must be <= ${MAX_SECRET_VALUE_LENGTH} characters`);
  }

  return value;
}
