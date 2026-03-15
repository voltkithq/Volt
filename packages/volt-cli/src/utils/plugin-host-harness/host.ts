import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { PluginIpcHost } from '../plugin-ipc-host.js';
import { ensurePluginHostBinary } from '../plugin-host-binary.js';
import { performScopedFsRequest } from './fs.js';
import { performStorageRequest } from './storage.js';
import type { PluginHarnessOptions, PluginHarnessState, RunningPluginHarness } from './types.js';

export async function startPluginHarness(
  options: PluginHarnessOptions,
): Promise<RunningPluginHarness> {
  const binary = await ensurePluginHostBinary();
  mkdirSync(options.dataRoot, { recursive: true });
  const host = new PluginIpcHost();
  const state: PluginHarnessState = {
    commands: new Set(),
    eventSubscriptions: new Set(),
    ipcHandlers: new Set(),
    emittedEvents: [],
  };
  const child = spawn(binary, ['--plugin', '--config', buildConfig(options)], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  host.attach(child);
  host.on('message', (message) => handleMessage(host, state, options, message as Record<string, unknown>));
  await host.waitForReady(10_000);

  return {
    process: child,
    state,
    async activate() {
      const response = await host.signal('activate');
      if (response.error) {
        throw new Error(response.error.message);
      }
    },
    async invokeCommand(id: string, args?: unknown) {
      const response = await host.request('plugin:invoke-command', { id, args: args ?? null });
      if (response.error) {
        throw new Error(response.error.message);
      }
      return response.payload;
    },
    async shutdown() {
      await host.shutdown(5_000);
    },
    kill() {
      host.kill();
    },
  };
}

export function createPluginHarnessDataRoot(pluginId: string): string {
  return mkdtempSync(join(tmpdir(), `${pluginId.replace(/\./g, '-')}-`));
}

function buildConfig(options: PluginHarnessOptions): string {
  return Buffer.from(
    JSON.stringify({
      pluginId: options.pluginId,
      backendEntry: options.backendEntry,
      manifest: options.manifest,
      capabilities: options.manifest.capabilities,
      dataRoot: options.dataRoot,
      delegatedGrants: [],
      hostIpcSettings: null,
    }),
  ).toString('base64');
}

function handleMessage(
  host: PluginIpcHost,
  state: PluginHarnessState,
  options: PluginHarnessOptions,
  message: Record<string, unknown>,
): void {
  if (message['type'] !== 'request') {
    return;
  }
  const id = String(message['id']);
  const method = String(message['method']);
  const payload = (message['payload'] ?? null) as Record<string, unknown> | null;
  try {
    host.sendResponse(id, method, routeRequest(state, options, method, payload));
  } catch (error) {
    host.sendError(
      id,
      method,
      'PLUGIN_TEST_HOST_ERROR',
      error instanceof Error ? error.message : String(error),
    );
  }
}

function routeRequest(
  state: PluginHarnessState,
  options: PluginHarnessOptions,
  method: string,
  payload: Record<string, unknown> | null,
): unknown {
  switch (method) {
    case 'plugin:register-command':
      state.commands.add(requireString(payload, 'id'));
      return null;
    case 'plugin:unregister-command':
      state.commands.delete(requireString(payload, 'id'));
      return null;
    case 'plugin:subscribe-event':
      state.eventSubscriptions.add(requireString(payload, 'event'));
      return null;
    case 'plugin:unsubscribe-event':
      state.eventSubscriptions.delete(requireString(payload, 'event'));
      return null;
    case 'plugin:register-ipc':
      state.ipcHandlers.add(requireString(payload, 'channel'));
      return null;
    case 'plugin:unregister-ipc':
      state.ipcHandlers.delete(requireString(payload, 'channel'));
      return null;
    case 'plugin:emit-event':
      state.emittedEvents.push({ event: requireString(payload, 'event'), data: payload?.['data'] ?? null });
      return null;
    case 'plugin:list-grants':
      return [];
    case 'plugin:bind-grant':
    case 'plugin:request-access':
      return null;
    default:
      if (method.startsWith('plugin:fs:')) {
        return performScopedFsRequest(options.dataRoot, method.slice('plugin:fs:'.length), payload);
      }
      if (method.startsWith('plugin:storage:')) {
        return performStorageRequest(options.dataRoot, method.slice('plugin:storage:'.length), payload);
      }
      if (method.startsWith('plugin:grant-fs:')) {
        return performScopedFsRequest(options.dataRoot, method.slice('plugin:grant-fs:'.length), payload);
      }
      throw new Error(`unsupported plugin host request '${method}'`);
  }
}

function requireString(payload: Record<string, unknown> | null, key: string): string {
  const value = payload?.[key];
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`payload is missing required '${key}' string`);
  }
  return value;
}
