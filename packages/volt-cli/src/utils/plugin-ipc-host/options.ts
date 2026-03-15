export interface PluginHostOptions {
  heartbeatIntervalMs?: number;
  heartbeatTimeoutMs?: number;
  callTimeoutMs?: number;
  maxInflight?: number;
  maxQueueDepth?: number;
}

export const DEFAULT_PLUGIN_HOST_OPTIONS = {
  heartbeatIntervalMs: 5000,
  heartbeatTimeoutMs: 3000,
  callTimeoutMs: 30000,
  maxInflight: 64,
  maxQueueDepth: 256,
} as const;
