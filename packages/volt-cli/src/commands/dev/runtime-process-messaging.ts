import type { ChildProcess } from 'node:child_process';
import {
  NATIVE_HOST_PROTOCOL_VERSION,
  type HostWindowCommandType,
  type NativeHostWindowConfig,
  type ParentToHostMessage,
} from '../native-host-protocol.js';

export interface HostProcessMessenger {
  sendStart(windowConfig: NativeHostWindowConfig): boolean;
  sendPing(pingId: number): boolean;
  sendEvalScript(jsId: string, script: string): void;
  sendWindowCommand(type: HostWindowCommandType, jsId: string): void;
  sendShutdown(): void;
}

export function createHostProcessMessenger(child: ChildProcess): HostProcessMessenger {
  return {
    sendStart(windowConfig: NativeHostWindowConfig): boolean {
      return sendHostMessage(child, {
        type: 'start',
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
        windowConfig,
      });
    },
    sendPing(pingId: number): boolean {
      return sendHostMessage(child, {
        type: 'ping',
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
        pingId,
      });
    },
    sendEvalScript(jsId: string, script: string): void {
      sendHostMessage(child, {
        type: 'eval-script',
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
        jsId,
        script,
      });
    },
    sendWindowCommand(type: HostWindowCommandType, jsId: string): void {
      sendHostMessage(child, {
        type,
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
        jsId,
      });
    },
    sendShutdown(): void {
      sendHostMessage(child, {
        type: 'shutdown',
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
      });
    },
  };
}

export function sendHostMessage(child: ChildProcess, message: ParentToHostMessage): boolean {
  if (typeof child.send !== 'function' || child.killed || !child.connected) {
    return false;
  }

  try {
    child.send(message);
    return true;
  } catch {
    return false;
  }
}
