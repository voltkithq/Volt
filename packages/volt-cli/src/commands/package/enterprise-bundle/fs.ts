import { mkdirSync, writeFileSync } from 'node:fs';

export function ensureDirectory(path: string, label: string): void {
  try {
    mkdirSync(path, { recursive: true });
  } catch (error) {
    throw new Error(
      `[volt] Failed to create ${label} directory at ${path}: ${toErrorMessage(error)}`,
      { cause: error },
    );
  }
}

export function safeWriteFile(path: string, contents: string): void {
  try {
    writeFileSync(path, contents, 'utf8');
  } catch (error) {
    throw new Error(
      `[volt] Failed to write enterprise bundle file ${path}: ${toErrorMessage(error)}`,
      {
        cause: error,
      },
    );
  }
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
