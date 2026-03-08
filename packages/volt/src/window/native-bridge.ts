import {
  windowClose,
  windowFocus,
  windowMaximize,
  windowMinimize,
  windowRestore,
  windowShow,
} from '@voltkit/volt-native';
import { getApp } from '../app.js';

interface NativeWindowCommandBridge {
  windowClose?(jsId: string): void;
  windowShow?(jsId: string): void;
  windowFocus?(jsId: string): void;
  windowMaximize?(jsId: string): void;
  windowMinimize?(jsId: string): void;
  windowRestore?(jsId: string): void;
}

export type NativeWindowCommandName = keyof Required<NativeWindowCommandBridge>;
export type NativeDispatchMode = 'runtime' | 'direct' | 'none';

const DIRECT_NATIVE_WINDOW_COMMANDS: Required<NativeWindowCommandBridge> = {
  windowClose,
  windowShow,
  windowFocus,
  windowMaximize,
  windowMinimize,
  windowRestore,
};

function getNativeWindowCommandBridge(): NativeWindowCommandBridge | null {
  try {
    const native = getApp().getNativeApp() as NativeWindowCommandBridge | null;
    if (native && typeof native === 'object') {
      return native;
    }
  } catch {
    // Framework app may not be initialized in isolated tests and scripts.
  }
  return null;
}

export function invokeNativeWindowCommand(
  command: NativeWindowCommandName,
  jsId: string,
): NativeDispatchMode {
  const runtimeBridge = getNativeWindowCommandBridge();
  const runtimeCommand = runtimeBridge?.[command];
  if (typeof runtimeCommand === 'function') {
    try {
      runtimeCommand.call(runtimeBridge, jsId);
      return 'runtime';
    } catch {
      // Fall back to direct native binding path below.
    }
  }

  try {
    DIRECT_NATIVE_WINDOW_COMMANDS[command](jsId);
    return 'direct';
  } catch {
    // Runtime may be unavailable; JS-side state/events remain authoritative fallback.
    return 'none';
  }
}
