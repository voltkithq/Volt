import {
  IpcContractValidationError,
  isIpcContractValidationError,
  type IpcRegistrar,
  type IpcSchema,
} from './types.js';

export function registerChannel(
  ipc: IpcRegistrar,
  channel: string,
  handler: (args: unknown) => Promise<unknown> | unknown,
): void {
  if (typeof ipc.hasHandler === 'function' && ipc.hasHandler(channel)) {
    throw new Error(`IPC handler already registered for channel: ${channel}`);
  }
  ipc.handle(channel, handler);
}

export function parseWithSchema<T>(
  schema: IpcSchema<T> | undefined,
  value: unknown,
  channel: string,
  phase: 'request' | 'response',
): T {
  if (!schema) {
    return value as T;
  }

  try {
    return schema.parse(value);
  } catch (error) {
    if (isIpcContractValidationError(error)) {
      throw error;
    }
    const message = error instanceof Error ? error.message : String(error);
    const schemaName = schema.name ? ` (${schema.name})` : '';
    throw new IpcContractValidationError(
      channel,
      phase,
      `IPC contract ${phase} validation failed for "${channel}"${schemaName}: ${message}`,
      error,
    );
  }
}
