export interface RuntimeBinding {
  event: string;
  callback: (payload: unknown) => void;
}

export interface RuntimeBridge {
  on(event: string, callback: (payload: unknown) => void): void;
  off(event: string, callback: (payload: unknown) => void): void;
}

export function replaceRuntimeBindings(
  bridge: RuntimeBridge,
  previous: RuntimeBinding[] | undefined,
  next: RuntimeBinding[],
): RuntimeBinding[] {
  if (Array.isArray(previous)) {
    for (const binding of previous) {
      bridge.off(binding.event, binding.callback);
    }
  }

  for (const binding of next) {
    bridge.on(binding.event, binding.callback);
  }

  return next;
}

export function clearRuntimeBindings(
  bridge: RuntimeBridge,
  bindings: RuntimeBinding[] | undefined,
): RuntimeBinding[] {
  if (Array.isArray(bindings)) {
    for (const binding of bindings) {
      bridge.off(binding.event, binding.callback);
    }
  }
  return [];
}
