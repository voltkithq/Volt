import { mkdirSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve } from 'node:path';
import type { NativeRuntimeBridge } from '../types.js';

type NativeScriptBridge = Pick<NativeRuntimeBridge, 'windowEvalScript'>;

interface DevRuntimeModuleState {
  projectRoot: string;
  defaultWindowId: string;
  nativeRuntime: NativeScriptBridge | null;
  permissions: string[];
}

const GLOBAL_RUNTIME_STATE_KEY = '__VOLT_DEV_RUNTIME_MODULE_STATE__';

function createDefaultRuntimeState(): DevRuntimeModuleState {
  return {
    projectRoot: process.cwd(),
    defaultWindowId: '',
    nativeRuntime: null,
    permissions: [],
  };
}

function resolveGlobalRuntimeState(): DevRuntimeModuleState {
  const globalScope = globalThis as Record<string, unknown>;
  const existing = globalScope[GLOBAL_RUNTIME_STATE_KEY];
  if (existing && typeof existing === 'object') {
    return existing as DevRuntimeModuleState;
  }
  const created = createDefaultRuntimeState();
  globalScope[GLOBAL_RUNTIME_STATE_KEY] = created;
  return created;
}

const runtimeState = resolveGlobalRuntimeState();

const DEV_DATA_DIR = '.volt-dev';

export function configureRuntimeModuleState(next: Partial<DevRuntimeModuleState>): void {
  if (typeof next.projectRoot === 'string' && next.projectRoot.trim().length > 0) {
    runtimeState.projectRoot = next.projectRoot;
  }
  if (typeof next.defaultWindowId === 'string') {
    runtimeState.defaultWindowId = next.defaultWindowId;
  }
  if (next.nativeRuntime !== undefined) {
    runtimeState.nativeRuntime = next.nativeRuntime;
  }
  if (Array.isArray(next.permissions)) {
    runtimeState.permissions = next.permissions
      .filter((permission): permission is string => typeof permission === 'string')
      .map((permission) => permission.trim())
      .filter((permission) => permission.length > 0);
  }
}

export function resetRuntimeModuleState(): void {
  runtimeState.projectRoot = process.cwd();
  runtimeState.defaultWindowId = '';
  runtimeState.nativeRuntime = null;
  runtimeState.permissions = [];
}

export function projectRoot(): string {
  return runtimeState.projectRoot;
}

export function runtimePermissions(): ReadonlySet<string> {
  return new Set(runtimeState.permissions);
}

export function devModuleError(moduleName: string, message: string): Error {
  return new Error(`[volt:${moduleName}] ${message}`);
}

export function ensureDevPermission(permission: string, apiName: string): void {
  if (!runtimePermissions().has(permission)) {
    throw devModuleError(
      permission,
      `Permission denied: ${apiName} requires '${permission}' in volt.config.ts permissions.`,
    );
  }
}

export function resolveProjectScopedPath(path: string, namespace: string): string {
  const trimmedPath = path.trim();
  if (trimmedPath.length === 0) {
    throw devModuleError('dev', 'Path must be a non-empty string.');
  }
  if (trimmedPath.includes('\0')) {
    throw devModuleError('dev', 'Path must not include null bytes.');
  }

  const safePath = trimmedPath.replace(/\\/g, '/').replace(/^\/+/, '');
  const baseDir = resolve(runtimeState.projectRoot, DEV_DATA_DIR, namespace);
  const absoluteTargetPath = resolve(baseDir, safePath);
  const relativePath = relative(baseDir, absoluteTargetPath);

  if (
    relativePath !== ''
    && (relativePath.startsWith('..') || isAbsolute(relativePath))
  ) {
    throw devModuleError('dev', `Path traversal is not allowed: ${path}`);
  }

  mkdirSync(dirname(absoluteTargetPath), { recursive: true });
  return absoluteTargetPath;
}

function serializeForScript(value: unknown): string {
  return JSON.stringify(value).replace(/<\//g, '<\\/');
}

function createEventDispatchScript(eventName: string, data: unknown): string {
  const eventLiteral = serializeForScript(eventName);
  const dataLiteral = serializeForScript(data ?? null);
  return `window.__volt_event__(${eventLiteral}, ${dataLiteral});`;
}

function resolveEventWindowId(windowId?: string): string {
  if (typeof windowId === 'string' && windowId.trim().length > 0) {
    return windowId.trim();
  }
  if (runtimeState.defaultWindowId.trim().length > 0) {
    return runtimeState.defaultWindowId.trim();
  }
  throw devModuleError('events', 'No target window ID is available for backend event dispatch.');
}

export function emitFrontendEvent(eventName: string, data?: unknown, windowId?: string): void {
  const event = eventName.trim();
  if (event.length === 0) {
    throw devModuleError('events', 'Event name must be a non-empty string.');
  }

  const nativeRuntime = runtimeState.nativeRuntime;
  if (!nativeRuntime) {
    throw devModuleError('events', 'Native runtime bridge is unavailable for backend event dispatch.');
  }

  const targetWindowId = resolveEventWindowId(windowId);
  nativeRuntime.windowEvalScript(
    targetWindowId,
    createEventDispatchScript(event, data ?? null),
  );
}
