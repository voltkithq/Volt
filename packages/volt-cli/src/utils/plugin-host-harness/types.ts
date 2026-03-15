import type { ChildProcess } from 'node:child_process';
import type { PluginManifest } from '../plugin-manifest.js';

export interface PluginHarnessOptions {
  pluginId: string;
  backendEntry: string;
  dataRoot: string;
  manifest: PluginManifest;
}

export interface PluginHarnessState {
  commands: Set<string>;
  eventSubscriptions: Set<string>;
  ipcHandlers: Set<string>;
  emittedEvents: Array<{ event: string; data: unknown }>;
}

export interface RunningPluginHarness {
  process: ChildProcess;
  state: PluginHarnessState;
  activate(): Promise<void>;
  invokeCommand(id: string, args?: unknown): Promise<unknown>;
  shutdown(): Promise<void>;
  kill(): void;
}
