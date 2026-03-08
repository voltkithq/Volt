import { createContractInvoker, createLegacyInvokeAdapter } from 'voltkit/renderer';
import { ipcCommands } from '../ipc-contract.js';

export interface VoltBridge {
  invoke(method: string, args?: unknown): Promise<unknown>;
  on(event: string, callback: (payload: unknown) => void): void;
  off(event: string, callback: (payload: unknown) => void): void;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

export function ensureBridge(): VoltBridge {
  if (!window.__volt__?.invoke || !window.__volt__.on || !window.__volt__.off) {
    throw new Error('window.__volt__ bridge is unavailable. Run this with `volt build`.');
  }
  return window.__volt__;
}

const invokeWithLegacyChannelAdapter = createLegacyInvokeAdapter(
  ipcCommands,
  async (channel, args) => ensureBridge().invoke(channel, args),
);

export const typedInvoke = createContractInvoker(
  ipcCommands,
  async (channel, args) => ensureBridge().invoke(channel, args),
);

export async function invoke<T>(method: string, args?: unknown): Promise<T> {
  return invokeWithLegacyChannelAdapter(method, args) as Promise<T>;
}
