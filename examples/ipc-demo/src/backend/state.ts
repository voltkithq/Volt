import * as voltCrypto from 'volt:crypto';

import { formatUuidFromHash, normalizeSecretKey, normalizeSecretValue } from '../backend-logic.js';

export const SHORTCUT_ACCELERATOR = 'CmdOrCtrl+Shift+P';
export const DB_PATH = 'ipc-demo/records.sqlite';
export const DEFAULT_SECRET_KEY = 'ipc-demo/demo-secret';

export const runtimeState = {
  databaseReady: false,
  menuConfigured: false,
  shortcutRegistered: false,
  trayReady: false,
};

export function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function buildUuidLike(seed: string): string {
  return formatUuidFromHash(voltCrypto.sha256(seed));
}

export function buildRecordId(seed: string): string {
  return buildUuidLike(`record:${seed}:${Math.random()}`);
}

export async function sleep(milliseconds: number): Promise<void> {
  await new Promise<void>((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}

export function parseSecretKeyPayload(data: unknown): string {
  const key = (data as { key?: unknown } | null)?.key;
  return normalizeSecretKey(key);
}

export function parseSecretSetPayload(data: unknown): { key: string; value: string } {
  const payload = (data as { key?: unknown; value?: unknown } | null) ?? {};
  return {
    key: normalizeSecretKey(payload.key),
    value: normalizeSecretValue(payload.value),
  };
}
