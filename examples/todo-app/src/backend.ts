import { ipcMain } from 'volt:ipc';
import * as voltCrypto from 'volt:crypto';
import * as voltDb from 'volt:db';
import { type TodoRecord, toTodoRecord } from './backend-logic.js';

const DB_PATH = 'todo-app/todos.sqlite';
let databaseReady = false;

function buildTodoId(seed: string): string {
  const hash = voltCrypto.sha256(`${seed}:${Date.now()}:${Math.random()}`);
  const hex = hash.length >= 32 ? hash.slice(0, 32) : hash.padEnd(32, '0');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
}

async function ensureDatabase(): Promise<void> {
  if (databaseReady) {
    return;
  }

  await voltDb.open(DB_PATH);
  await voltDb.execute(
    `CREATE TABLE IF NOT EXISTS todos (
      id TEXT PRIMARY KEY,
      text TEXT NOT NULL,
      completed INTEGER NOT NULL DEFAULT 0,
      created_at INTEGER NOT NULL
    )`,
  );
  databaseReady = true;
}

async function listTodos(): Promise<TodoRecord[]> {
  await ensureDatabase();
  const rows = await voltDb.query(
    'SELECT id, text, completed, created_at FROM todos ORDER BY created_at DESC',
  );
  if (!Array.isArray(rows)) {
    return [];
  }
  return rows.map(toTodoRecord);
}

ipcMain.handle('get-todos', async () => listTodos());

ipcMain.handle('add-todo', async (args: unknown) => {
  await ensureDatabase();
  const textRaw = (args as { text?: unknown })?.text;
  if (typeof textRaw !== 'string') {
    throw new Error('add-todo.text must be a string');
  }
  const text = textRaw.trim();
  if (!text) {
    throw new Error('todo text must not be empty');
  }

  const todo: TodoRecord = {
    id: buildTodoId(text),
    text,
    completed: false,
    createdAt: Date.now(),
  };
  await voltDb.execute(
    'INSERT INTO todos (id, text, completed, created_at) VALUES (?, ?, ?, ?)',
    [todo.id, todo.text, 0, todo.createdAt],
  );

  return todo;
});

ipcMain.handle('toggle-todo', async (args: unknown) => {
  await ensureDatabase();
  const id = (args as { id?: unknown })?.id;
  if (typeof id !== 'string' || !id.trim()) {
    throw new Error('toggle-todo.id must be a non-empty string');
  }

  const updateResult = await voltDb.execute(
    `UPDATE todos
     SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END
     WHERE id = ?`,
    [id],
  );
  const rowsAffected = Number((updateResult as { rowsAffected?: unknown })?.rowsAffected ?? 0);
  if (!Number.isFinite(rowsAffected) || rowsAffected < 1) {
    throw new Error(`Todo not found: ${id}`);
  }

  const row = await voltDb.queryOne(
    'SELECT id, text, completed, created_at FROM todos WHERE id = ?',
    [id],
  );
  if (!row || typeof row !== 'object') {
    throw new Error(`Todo not found after toggle: ${id}`);
  }

  return toTodoRecord(row);
});

ipcMain.handle('delete-todo', async (args: unknown) => {
  await ensureDatabase();
  const id = (args as { id?: unknown })?.id;
  if (typeof id !== 'string' || !id.trim()) {
    throw new Error('delete-todo.id must be a non-empty string');
  }

  const result = await voltDb.execute('DELETE FROM todos WHERE id = ?', [id]);
  const rowsAffected = Number((result as { rowsAffected?: unknown })?.rowsAffected ?? 0);
  if (!Number.isFinite(rowsAffected) || rowsAffected < 1) {
    throw new Error(`Todo not found: ${id}`);
  }
  return { success: true };
});
