type PluginLifecycleEventName = 'plugin:lifecycle' | 'plugin:failed' | 'plugin:activated';
type PluginEventHandler = (event: unknown) => void;

const listeners = new Map<PluginLifecycleEventName, Set<PluginEventHandler>>();

function normalizePluginId(pluginId: string): string {
  const normalized = pluginId.trim();
  if (!normalized) {
    throw new Error('plugin id must be a non-empty string');
  }
  return normalized;
}

function normalizeGrantId(grantId: string): string {
  const normalized = grantId.trim();
  if (!normalized) {
    throw new Error('grant id must be a non-empty string');
  }
  return normalized;
}

function normalizeSurface(surface: string): string {
  const normalized = surface.trim();
  if (!normalized) {
    throw new Error('surface must be a non-empty string');
  }
  return normalized;
}

function normalizeEventName(eventName: string): PluginLifecycleEventName {
  if (
    eventName === 'plugin:lifecycle'
    || eventName === 'plugin:failed'
    || eventName === 'plugin:activated'
  ) {
    return eventName;
  }
  throw new Error(`unsupported plugin event '${eventName}'`);
}

export async function delegateGrant(pluginId: string, grantId: string): Promise<void> {
  normalizePluginId(pluginId);
  normalizeGrantId(grantId);
}

export async function revokeGrant(pluginId: string, grantId: string): Promise<void> {
  normalizePluginId(pluginId);
  normalizeGrantId(grantId);
}

export async function prefetchFor(surface: string): Promise<void> {
  normalizeSurface(surface);
}

export async function getStates(): Promise<unknown[]> {
  return [];
}

export async function getPluginState(pluginId: string): Promise<unknown | null> {
  normalizePluginId(pluginId);
  return null;
}

export async function getErrors(): Promise<unknown[]> {
  return [];
}

export async function getPluginErrors(pluginId: string): Promise<unknown[]> {
  normalizePluginId(pluginId);
  return [];
}

export async function getDiscoveryIssues(): Promise<unknown[]> {
  return [];
}

export async function retryPlugin(pluginId: string): Promise<void> {
  normalizePluginId(pluginId);
}

export async function enablePlugin(pluginId: string): Promise<void> {
  normalizePluginId(pluginId);
}

export function on(eventName: string, handler: PluginEventHandler): void {
  const normalized = normalizeEventName(eventName);
  const existing = listeners.get(normalized);
  if (existing) {
    existing.add(handler);
    return;
  }
  listeners.set(normalized, new Set([handler]));
}

export function off(eventName: string, handler: PluginEventHandler): void {
  const normalized = normalizeEventName(eventName);
  listeners.get(normalized)?.delete(handler);
}
