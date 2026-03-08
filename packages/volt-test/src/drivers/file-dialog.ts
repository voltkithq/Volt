import { isAbsolute, posix, win32 } from 'node:path';

export type FileDialogAutomationPlatform = 'win32' | 'darwin' | 'linux';

export interface OpenDialogAutomationResult {
  canceled: boolean;
  filePaths: string[];
}

export interface SaveDialogAutomationResult {
  canceled: boolean;
  filePath: string;
}

export interface FileDialogAutomationDriverOptions {
  platform?: FileDialogAutomationPlatform;
}

export class FileDialogAutomationDriver {
  private readonly platform: FileDialogAutomationPlatform;

  public constructor(options: FileDialogAutomationDriverOptions = {}) {
    this.platform = resolveAutomationPlatform(options.platform ?? process.platform);
  }

  public getPlatform(): FileDialogAutomationPlatform {
    return this.platform;
  }

  public parseOpenDialogResult(payload: unknown): OpenDialogAutomationResult {
    const value = asRecord(payload);
    if (!value) {
      throw new Error('[volt:test] invalid open dialog payload: expected object.');
    }

    const canceled = value.canceled;
    if (typeof canceled !== 'boolean') {
      throw new Error('[volt:test] invalid open dialog payload: missing canceled boolean.');
    }

    const rawFilePaths = value.filePaths;
    if (!Array.isArray(rawFilePaths) || rawFilePaths.some((entry) => typeof entry !== 'string')) {
      throw new Error('[volt:test] invalid open dialog payload: filePaths must be a string array.');
    }

    const filePaths = rawFilePaths.map((entry) => this.normalizePath(entry));
    if (canceled && filePaths.length > 0) {
      throw new Error('[volt:test] invalid open dialog payload: canceled dialog must not return file paths.');
    }

    if (!canceled && filePaths.some((entry) => entry.length === 0)) {
      throw new Error('[volt:test] invalid open dialog payload: selected file paths must be non-empty.');
    }

    return { canceled, filePaths };
  }

  public parseSaveDialogResult(payload: unknown): SaveDialogAutomationResult {
    const value = asRecord(payload);
    if (!value) {
      throw new Error('[volt:test] invalid save dialog payload: expected object.');
    }

    const canceled = value.canceled;
    if (typeof canceled !== 'boolean') {
      throw new Error('[volt:test] invalid save dialog payload: missing canceled boolean.');
    }

    const rawFilePath = value.filePath;
    if (typeof rawFilePath !== 'string') {
      throw new Error('[volt:test] invalid save dialog payload: missing filePath string.');
    }

    const filePath = this.normalizePath(rawFilePath);
    if (!canceled && filePath.length === 0) {
      throw new Error('[volt:test] invalid save dialog payload: filePath must be non-empty when canceled=false.');
    }

    if (canceled && filePath.length > 0) {
      throw new Error('[volt:test] invalid save dialog payload: canceled dialog must not return filePath.');
    }

    return { canceled, filePath };
  }

  public normalizePath(pathValue: string): string {
    const trimmed = pathValue.trim();
    if (trimmed.length === 0) {
      return '';
    }

    if (this.platform === 'win32') {
      const withWinSeparators = trimmed.replace(/\//g, '\\');
      const normalized = win32.normalize(withWinSeparators);
      if (isWindowsDrivePath(normalized)) {
        return `${normalized.slice(0, 1).toUpperCase()}${normalized.slice(1)}`;
      }
      return normalized;
    }

    const withPosixSeparators = trimmed.replace(/\\/g, '/');
    return posix.normalize(withPosixSeparators);
  }

  public assertOpenSelection(
    result: OpenDialogAutomationResult,
    expectedAbsolutePaths: readonly string[],
  ): void {
    if (result.canceled) {
      throw new Error('[volt:test] expected file selection but dialog was canceled.');
    }

    const normalizedExpected = expectedAbsolutePaths.map((entry) => this.normalizePath(entry));
    const normalizedActual = result.filePaths.map((entry) => this.normalizePath(entry));
    if (normalizedExpected.length !== normalizedActual.length) {
      throw new Error(
        `[volt:test] expected ${normalizedExpected.length} selected files, got ${normalizedActual.length}.`,
      );
    }

    for (let index = 0; index < normalizedExpected.length; index += 1) {
      const expectedPath = normalizedExpected[index];
      const actualPath = normalizedActual[index];
      if (actualPath !== expectedPath) {
        throw new Error(
          `[volt:test] selected file mismatch at index ${index}: expected "${expectedPath}", got "${actualPath}".`,
        );
      }
      if (!isAbsolute(actualPath)) {
        throw new Error(`[volt:test] expected absolute file path but got "${actualPath}".`);
      }
    }
  }

  public assertSaveSelection(result: SaveDialogAutomationResult, expectedAbsolutePath: string): void {
    if (result.canceled) {
      throw new Error('[volt:test] expected save path but dialog was canceled.');
    }

    const normalizedExpected = this.normalizePath(expectedAbsolutePath);
    const normalizedActual = this.normalizePath(result.filePath);
    if (normalizedActual !== normalizedExpected) {
      throw new Error(
        `[volt:test] save file mismatch: expected "${normalizedExpected}", got "${normalizedActual}".`,
      );
    }

    if (!isAbsolute(normalizedActual)) {
      throw new Error(`[volt:test] expected absolute save path but got "${normalizedActual}".`);
    }
  }
}

function resolveAutomationPlatform(platformValue: string): FileDialogAutomationPlatform {
  if (platformValue === 'win32' || platformValue === 'darwin' || platformValue === 'linux') {
    return platformValue;
  }
  throw new Error(`[volt:test] unsupported file-dialog automation platform: ${platformValue}`);
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

function isWindowsDrivePath(value: string): boolean {
  return /^[a-zA-Z]:\\/.test(value);
}
