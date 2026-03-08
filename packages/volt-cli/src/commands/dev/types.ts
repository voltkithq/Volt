import type { ChildProcess } from 'node:child_process';
import type { BrowserWindow } from 'voltkit';
import type {
  HostToParentMessage,
  NativeHostWindowConfig,
  ParentToHostMessage,
} from '../native-host-protocol.js';

export type RuntimeMode = 'main-thread-macos' | 'split-runtime-threaded';

export interface NativeBinding {
  VoltApp: new (config: NativeHostWindowConfig) => {
    createWindow(config: NativeHostWindowConfig): void;
    onEvent(callback: (...args: unknown[]) => void): void;
    run(): void;
  };
  windowEvalScript(jsId: string, script: string): void;
  windowClose(jsId: string): void;
  windowShow(jsId: string): void;
  windowFocus(jsId: string): void;
  windowMaximize(jsId: string): void;
  windowMinimize(jsId: string): void;
  windowRestore(jsId: string): void;
}

export interface NativeRuntimeBridge {
  onEvent(callback: (eventJson: string) => void): void;
  windowEvalScript(jsId: string, script: string): void;
  windowClose(jsId: string): void;
  windowShow(jsId: string): void;
  windowFocus(jsId: string): void;
  windowMaximize(jsId: string): void;
  windowMinimize(jsId: string): void;
  windowRestore(jsId: string): void;
  run(): Promise<void>;
  shutdown(): void;
}

export interface OutOfProcessRuntimeOptions {
  hostPath?: string;
  startupTimeoutMs?: number;
  heartbeatIntervalMs?: number;
  heartbeatTimeoutMs?: number;
  env?: NodeJS.ProcessEnv;
  pipeOutput?: boolean;
  onSpawn?: (pid: number) => void;
}

export interface NativeIpcEvent {
  type: 'ipc-message';
  windowId: string;
  raw: unknown;
}

export interface NativeQuitEvent {
  type: 'quit';
}

export interface NativeMenuEvent {
  type: 'menu-event';
  menuId: string;
}

export interface NativeShortcutEvent {
  type: 'shortcut-triggered';
  id: number;
}

export interface NativeWindowClosedEvent {
  type: 'window-closed';
  windowId: string;
  jsWindowId: string | null;
}

export type NativeRuntimeEvent =
  | NativeIpcEvent
  | NativeQuitEvent
  | NativeMenuEvent
  | NativeShortcutEvent
  | NativeWindowClosedEvent;

export interface NativeIpcRequest {
  id: string;
  method: string;
  args?: unknown;
}

export interface IpcMessageHandlingOptions {
  timeoutMs?: number;
  maxPayloadBytes?: number;
  maxInFlightPerWindow?: number;
}

export interface RuntimeProtocolDependencies {
  childProcessFork: typeof import('node:child_process').fork;
  fileUrlToPath: typeof import('node:url').fileURLToPath;
  createUrl: (path: string, base: string) => URL;
  protocolVersion: number;
}

export type ResolveWindowByJsId = (
  jsWindowId: string,
) => Pick<BrowserWindow, 'destroy'> | undefined;

export type HostMessageGuard = (raw: unknown) => raw is HostToParentMessage;
export type HostMessageSender = (child: ChildProcess, message: ParentToHostMessage) => boolean;
