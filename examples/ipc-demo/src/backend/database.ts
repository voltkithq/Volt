import * as voltDb from 'volt:db';

import type { DbRecord } from '../backend-logic.js';
import { toDbRecords } from '../backend-logic.js';

import { buildRecordId, DB_PATH, runtimeState } from './state.js';

export async function ensureDatabase(): Promise<void> {
  if (runtimeState.databaseReady) {
    return;
  }

  await voltDb.open(DB_PATH);
  await voltDb.execute(
    `CREATE TABLE IF NOT EXISTS demo_records (
      id TEXT PRIMARY KEY,
      message TEXT NOT NULL,
      created_at INTEGER NOT NULL
    )`,
  );
  runtimeState.databaseReady = true;
}

export async function insertDbRecord(message: string): Promise<DbRecord> {
  await ensureDatabase();
  const trimmed = message.trim();
  if (!trimmed) {
    throw new Error('db message must not be empty');
  }

  const record: DbRecord = {
    id: buildRecordId(trimmed),
    message: trimmed,
    createdAt: Date.now(),
  };
  await voltDb.execute('INSERT INTO demo_records (id, message, created_at) VALUES (?, ?, ?)', [
    record.id,
    record.message,
    record.createdAt,
  ]);
  return record;
}

export async function listDbRecords(): Promise<DbRecord[]> {
  await ensureDatabase();
  const rows = await voltDb.query(
    'SELECT id, message, created_at FROM demo_records ORDER BY created_at DESC LIMIT 12',
  );
  return toDbRecords(rows);
}
