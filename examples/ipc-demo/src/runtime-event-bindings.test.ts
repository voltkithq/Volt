import { describe, expect, it, vi } from 'vitest';
import {
  clearRuntimeBindings,
  replaceRuntimeBindings,
  type RuntimeBinding,
  type RuntimeBridge,
} from './runtime-event-bindings.js';

interface RuntimeBridgeMock extends RuntimeBridge {
  on: ReturnType<typeof vi.fn<(event: string, callback: (payload: unknown) => void) => void>>;
  off: ReturnType<typeof vi.fn<(event: string, callback: (payload: unknown) => void) => void>>;
}

function createBridge(): RuntimeBridgeMock {
  return {
    on: vi.fn<(event: string, callback: (payload: unknown) => void) => void>(),
    off: vi.fn<(event: string, callback: (payload: unknown) => void) => void>(),
  };
}

function createBinding(event: string): RuntimeBinding {
  return {
    event,
    callback: () => undefined,
  };
}

describe('runtime event bindings', () => {
  it('replaces previous bindings before registering new ones', () => {
    const bridge = createBridge();
    const previous = [createBinding('demo:progress'), createBinding('demo:shortcut')];
    const next = [createBinding('demo:tray-click')];

    const result = replaceRuntimeBindings(bridge, previous, next);

    expect(result).toBe(next);
    expect(bridge.off).toHaveBeenCalledTimes(2);
    expect(bridge.off).toHaveBeenCalledWith('demo:progress', previous[0].callback);
    expect(bridge.off).toHaveBeenCalledWith('demo:shortcut', previous[1].callback);
    expect(bridge.on).toHaveBeenCalledTimes(1);
    expect(bridge.on).toHaveBeenCalledWith('demo:tray-click', next[0].callback);
  });

  it('registers new bindings when no previous bindings exist', () => {
    const bridge = createBridge();
    const next = [createBinding('demo:native-ready')];

    replaceRuntimeBindings(bridge, undefined, next);

    expect(bridge.off).not.toHaveBeenCalled();
    expect(bridge.on).toHaveBeenCalledTimes(1);
    expect(bridge.on).toHaveBeenCalledWith('demo:native-ready', next[0].callback);
  });

  it('clears bindings and returns an empty binding list', () => {
    const bridge = createBridge();
    const bindings = [createBinding('demo:db-updated'), createBinding('demo:native-error')];

    const result = clearRuntimeBindings(bridge, bindings);

    expect(result).toEqual([]);
    expect(bridge.off).toHaveBeenCalledTimes(2);
    expect(bridge.off).toHaveBeenCalledWith('demo:db-updated', bindings[0].callback);
    expect(bridge.off).toHaveBeenCalledWith('demo:native-error', bindings[1].callback);
  });
});
