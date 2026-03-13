import { createRequire } from 'node:module';
import type { NativeHostWindowConfig } from '../native-host-protocol.js';
import type {
  NativeBinding,
  NativeRuntimeBridge,
  RuntimeMode,
} from './types.js';
import {
  startInProcessRuntime,
  startOutOfProcessRuntime,
} from './runtime-lifecycle.js';

export { isHostToParentMessage } from './runtime-protocol.js';
export { startInProcessRuntime, startOutOfProcessRuntime } from './runtime-lifecycle.js';

export function runtimeModeForPlatform(platform: NodeJS.Platform): RuntimeMode {
  return platform === 'darwin' ? 'main-thread-macos' : 'split-runtime-threaded';
}

export function currentRuntimeMode(): RuntimeMode {
  return runtimeModeForPlatform(process.platform);
}

export function shouldUseOutOfProcessNativeHost(
  env: NodeJS.ProcessEnv = process.env,
): boolean {
  const raw = env.VOLT_NATIVE_HOST?.trim().toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes';
}

export async function startNativeRuntime(
  native: NativeBinding,
  windowConfig: NativeHostWindowConfig,
): Promise<NativeRuntimeBridge> {
  if (!shouldUseOutOfProcessNativeHost()) {
    return startInProcessRuntime(native, windowConfig);
  }

  console.warn(
    '[volt] VOLT_NATIVE_HOST is enabled; out-of-process host mode is experimental and may not support all native main-process APIs.',
  );

  try {
    return await startOutOfProcessRuntime(windowConfig);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.warn(
      `[volt] Native host process failed, falling back to in-process runtime: ${message}`,
    );
    return startInProcessRuntime(native, windowConfig);
  }
}

export function loadNativeBinding(): NativeBinding | null {
  try {
    const require = createRequire(import.meta.url);
    return require('@voltkit/volt-native') as NativeBinding;
  } catch {
    // Fallback: resolve from project root (needed when volt-cli is file-linked)
    try {
      const require = createRequire(new URL('file:///' + process.cwd().replace(/\\/g, '/') + '/package.json'));
      return require('@voltkit/volt-native') as NativeBinding;
    } catch {
      return null;
    }
  }
}
