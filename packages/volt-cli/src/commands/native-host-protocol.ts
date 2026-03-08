import type { Permission } from 'voltkit';

export const NATIVE_HOST_PROTOCOL_VERSION = 1;

export interface NativeHostWindowConfig {
  name: string;
  permissions: Permission[];
  jsId: string;
  url: string;
  devtools: boolean;
  window: {
    title: string;
    width: number;
    height: number;
    minWidth?: number;
    minHeight?: number;
    resizable: boolean;
    decorations: boolean;
  };
}

export interface HostStartMessage {
  type: 'start';
  protocolVersion: number;
  windowConfig: NativeHostWindowConfig;
}

export interface HostEvalScriptMessage {
  type: 'eval-script';
  protocolVersion: number;
  jsId: string;
  script: string;
}

export type HostWindowCommandType =
  | 'close-window'
  | 'show-window'
  | 'focus-window'
  | 'maximize-window'
  | 'minimize-window'
  | 'restore-window';

export interface HostWindowCommandMessage {
  type: HostWindowCommandType;
  protocolVersion: number;
  jsId: string;
}

export interface HostShutdownMessage {
  type: 'shutdown';
  protocolVersion: number;
}

export interface HostPingMessage {
  type: 'ping';
  protocolVersion: number;
  pingId: number;
}

export type ParentToHostMessage =
  | HostStartMessage
  | HostEvalScriptMessage
  | HostWindowCommandMessage
  | HostShutdownMessage
  | HostPingMessage;

export interface HostStartingMessage {
  type: 'starting';
  protocolVersion: number;
}

export interface HostReadyMessage {
  type: 'ready';
  protocolVersion: number;
}

export interface HostEventMessage {
  type: 'event';
  protocolVersion: number;
  eventJson: string;
}

export interface HostRuntimeErrorMessage {
  type: 'runtime-error';
  protocolVersion: number;
  message: string;
}

export interface HostNativeUnavailableMessage {
  type: 'native-unavailable';
  protocolVersion: number;
  message: string;
}

export interface HostPongMessage {
  type: 'pong';
  protocolVersion: number;
  pingId: number;
}

export interface HostStoppingMessage {
  type: 'stopping';
  protocolVersion: number;
}

export interface HostStoppedMessage {
  type: 'stopped';
  protocolVersion: number;
  code?: number;
}

export type HostToParentMessage =
  | HostStartingMessage
  | HostReadyMessage
  | HostEventMessage
  | HostRuntimeErrorMessage
  | HostNativeUnavailableMessage
  | HostPongMessage
  | HostStoppingMessage
  | HostStoppedMessage;
