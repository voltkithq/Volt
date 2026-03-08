import { describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/dev.js';

describe('runtime mode contract', () => {
  it('maps macOS to main-thread runtime mode', () => {
    expect(__testOnly.runtimeModeForPlatform('darwin')).toBe('main-thread-macos');
  });

  it('maps non-macOS platforms to split runtime mode', () => {
    expect(__testOnly.runtimeModeForPlatform('linux')).toBe('split-runtime-threaded');
    expect(__testOnly.runtimeModeForPlatform('win32')).toBe('split-runtime-threaded');
    expect(__testOnly.runtimeModeForPlatform('freebsd')).toBe('split-runtime-threaded');
    expect(__testOnly.runtimeModeForPlatform('aix')).toBe('split-runtime-threaded');
    expect(__testOnly.runtimeModeForPlatform('android')).toBe('split-runtime-threaded');
  });

  it('reports current platform runtime mode', () => {
    const expected = process.platform === 'darwin' ? 'main-thread-macos' : 'split-runtime-threaded';
    expect(__testOnly.currentRuntimeMode()).toBe(expected);
  });

  it('uses in-process native runtime by default', () => {
    expect(__testOnly.shouldUseOutOfProcessNativeHost({} as NodeJS.ProcessEnv)).toBe(false);
    expect(__testOnly.shouldUseOutOfProcessNativeHost({ VOLT_NATIVE_HOST: '0' } as NodeJS.ProcessEnv)).toBe(false);
  });

  it('enables out-of-process native host only when explicitly requested', () => {
    expect(__testOnly.shouldUseOutOfProcessNativeHost({ VOLT_NATIVE_HOST: '1' } as NodeJS.ProcessEnv)).toBe(true);
    expect(__testOnly.shouldUseOutOfProcessNativeHost({ VOLT_NATIVE_HOST: 'true' } as NodeJS.ProcessEnv)).toBe(
      true,
    );
    expect(__testOnly.shouldUseOutOfProcessNativeHost({ VOLT_NATIVE_HOST: 'yes' } as NodeJS.ProcessEnv)).toBe(
      true,
    );
  });

  it('parses valid dev server ports', () => {
    expect(__testOnly.parseDevPort('3000')).toBe(3000);
    expect(__testOnly.parseDevPort('65535')).toBe(65535);
  });

  it('rejects invalid dev server ports', () => {
    expect(() => __testOnly.parseDevPort('0')).toThrow('Invalid --port value');
    expect(() => __testOnly.parseDevPort('70000')).toThrow('Invalid --port value');
    expect(() => __testOnly.parseDevPort('abc')).toThrow('Invalid --port value');
  });
});
