import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { DatabaseSync, type SQLInputValue } from 'node:sqlite';
import { devModuleError, resolveProjectScopedPath } from './shared.js';

interface ExecuteResult {
  rowsAffected: number;
}

let connection: DatabaseSync | null = null;
let openPath: string | null = null;

function ensureConnection(): DatabaseSync {
  if (!connection) {
    throw devModuleError('db', 'Database is not open. Call db.open(path) first.');
  }
  return connection;
}

function normalizeSql(sql: string): string {
  const statement = sql.trim();
  if (!statement) {
    throw devModuleError('db', 'SQL statement must be a non-empty string.');
  }
  return statement;
}

function normalizeBindValue(value: unknown): SQLInputValue {
  if (value === undefined) {
    return null;
  }
  if (value instanceof Uint8Array) {
    return Buffer.from(value);
  }
  if (Array.isArray(value) || (value && typeof value === 'object')) {
    return JSON.stringify(value);
  }
  return value as SQLInputValue;
}

function normalizeBindParams(params?: unknown[]): SQLInputValue[] {
  if (!Array.isArray(params)) {
    return [];
  }
  return params.map((value) => normalizeBindValue(value));
}

function toJsonValue(value: unknown): unknown {
  if (value === undefined) {
    return null;
  }
  if (value === null) {
    return null;
  }
  if (typeof value === 'bigint') {
    const asNumber = Number(value);
    return Number.isSafeInteger(asNumber) ? asNumber : value.toString();
  }
  if (Buffer.isBuffer(value) || value instanceof Uint8Array) {
    return Buffer.from(value).toString('base64');
  }
  return value;
}

function toJsonRow(row: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(row)) {
    result[key] = toJsonValue(value);
  }
  return result;
}

function resolveDatabasePath(path: string): string {
  const trimmedPath = path.trim();
  if (!trimmedPath) {
    throw devModuleError('db', 'Database path must be a non-empty string.');
  }
  if (trimmedPath === ':memory:') {
    return trimmedPath;
  }
  const resolvedPath = resolveProjectScopedPath(trimmedPath, 'db');
  mkdirSync(dirname(resolvedPath), { recursive: true });
  return resolvedPath;
}

function configureConnection(db: DatabaseSync): void {
  db.exec('PRAGMA journal_mode = WAL;');
  db.exec('PRAGMA foreign_keys = ON;');
  db.exec('PRAGMA busy_timeout = 5000;');
}

export async function open(path: string): Promise<void> {
  const databasePath = resolveDatabasePath(path);
  if (connection && openPath === databasePath) {
    return;
  }
  if (connection) {
    connection.close();
  }
  connection = new DatabaseSync(databasePath);
  openPath = databasePath;
  configureConnection(connection);
}

export async function close(): Promise<void> {
  if (!connection) {
    return;
  }
  connection.close();
  connection = null;
  openPath = null;
}

export async function execute(sql: string, params?: unknown[]): Promise<ExecuteResult> {
  const db = ensureConnection();
  const statement = db.prepare(normalizeSql(sql));
  const result = statement.run(...normalizeBindParams(params));
  const rowsAffected = Number((result as { changes?: unknown }).changes ?? 0);
  return { rowsAffected: Number.isFinite(rowsAffected) ? rowsAffected : 0 };
}

export async function query(sql: string, params?: unknown[]): Promise<unknown[]> {
  const db = ensureConnection();
  const statement = db.prepare(normalizeSql(sql));
  const rows = statement.all(...normalizeBindParams(params));
  if (!Array.isArray(rows)) {
    return [];
  }
  return rows.map((row) => toJsonRow((row ?? {}) as Record<string, unknown>));
}

export async function queryOne(sql: string, params?: unknown[]): Promise<unknown | null> {
  const db = ensureConnection();
  const statement = db.prepare(normalizeSql(sql));
  const row = statement.get(...normalizeBindParams(params));
  if (!row || typeof row !== 'object') {
    return null;
  }
  return toJsonRow(row as Record<string, unknown>);
}

export async function transaction<T>(callback: () => Promise<T> | T): Promise<T> {
  const db = ensureConnection();
  db.exec('BEGIN IMMEDIATE');
  try {
    const result = await callback();
    db.exec('COMMIT');
    return result;
  } catch (error) {
    try {
      db.exec('ROLLBACK');
    } catch {
      // Keep the original transaction failure as the thrown error.
    }
    throw error;
  }
}
