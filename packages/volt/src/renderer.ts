/**
 * Renderer-safe entrypoint.
 * Exposes only APIs that are valid in the WebView/browser context.
 */
export { invoke, on, off } from './ipc.js';
export type { IpcErrorCode } from './ipc.js';
export { data } from './data.js';
export { workflow } from './workflow.js';
export type { DataProfile, DataProfileOptions, DataQueryOptions, DataQueryResult } from './data.js';
export type { WorkflowPluginInfo, WorkflowRunOptions, WorkflowRunResult } from './workflow.js';
export {
  createContractInvoker,
  createLegacyInvokeAdapter,
  createSchema,
  defineCommands,
  IpcSchema,
  isIpcContractValidationError,
  resolveContractChannel,
  IpcContractValidationError,
} from './ipc-contract.js';
export type {
  InferCommandRequest,
  InferCommandResponse,
  InferSchemaValue,
  IpcCommandDefinition,
  IpcCommandMap,
  IpcInvokeFn,
  IpcSchema as IpcSchemaType,
  TypedIpcInvoker,
} from './ipc-contract.js';
