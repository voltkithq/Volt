export type {
  ContractHandlers,
  InferCommandRequest,
  InferCommandResponse,
  InferSchemaValue,
  IpcCommandDefinition,
  IpcCommandMap,
  IpcInvokeFn,
  IpcRegistrar,
  TypedIpcInvoker,
} from './ipc-contract/types.js';

import { IpcSchema as IpcSchemaValue, createSchema } from './ipc-contract/schema.js';
import type { IpcSchema as IpcSchemaType } from './ipc-contract/types.js';

export {
  IpcContractValidationError,
  isIpcContractValidationError,
} from './ipc-contract/types.js';

export type IpcSchema<T> = IpcSchemaType<T>;
export const IpcSchema = IpcSchemaValue;
export { createSchema };
export { defineCommands } from './ipc-contract/define.js';
export { resolveContractChannel } from './ipc-contract/aliases.js';
export { registerContractHandlers } from './ipc-contract/handlers.js';
export {
  createContractInvoker,
  createLegacyInvokeAdapter,
} from './ipc-contract/invoker.js';
