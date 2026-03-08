export interface IpcSchema<T> {
  /** Human-readable schema name used in validation errors. */
  readonly name?: string;
  /** Parse and validate a value. Throw when validation fails. */
  parse(value: unknown): T;
}

export type InferSchemaValue<TSchema> = TSchema extends IpcSchema<infer TValue> ? TValue : never;

export interface IpcCommandDefinition<TRequest = unknown, TResponse = unknown> {
  /** Request payload schema (renderer -> backend). */
  request?: IpcSchema<TRequest>;
  /** Response payload schema (backend -> renderer). */
  response?: IpcSchema<TResponse>;
  /** Legacy channel names that should map to this command key. */
  aliases?: readonly string[];
}

export type IpcCommandMap = Record<string, IpcCommandDefinition<unknown, unknown>>;

export type InferCommandRequest<TCommand> =
  TCommand extends IpcCommandDefinition<infer TRequest, unknown>
    ? TRequest
    : never;

export type InferCommandResponse<TCommand> =
  TCommand extends IpcCommandDefinition<unknown, infer TResponse>
    ? TResponse
    : never;

export type ContractHandlers<TCommands extends IpcCommandMap> = {
  [TKey in keyof TCommands]: (
    args: InferCommandRequest<TCommands[TKey]>,
  ) => Promise<InferCommandResponse<TCommands[TKey]>> | InferCommandResponse<TCommands[TKey]>;
};

export interface IpcRegistrar {
  handle(channel: string, handler: (args: unknown) => Promise<unknown> | unknown): void;
  hasHandler?(channel: string): boolean;
}

export type IpcInvokeFn = (channel: string, args: unknown) => Promise<unknown>;

export interface TypedIpcInvoker<TCommands extends IpcCommandMap> {
  invoke<TKey extends keyof TCommands & string>(
    channel: TKey,
    args: InferCommandRequest<TCommands[TKey]>,
  ): Promise<InferCommandResponse<TCommands[TKey]>>;
  /**
   * Compatibility path for untyped, legacy callers.
   * Legacy aliases are resolved to canonical command keys before dispatch.
   */
  invokeLegacy(channel: string, args: unknown): Promise<unknown>;
  /** Resolve a legacy alias to its canonical command key. */
  resolveChannel(channel: string): string;
}

export class IpcContractValidationError extends Error {
  readonly code = 'IPC_CONTRACT_VALIDATION_ERROR' as const;

  constructor(
    readonly channel: string,
    readonly phase: 'request' | 'response',
    message: string,
    readonly details?: unknown,
  ) {
    super(message);
    this.name = 'IpcContractValidationError';
  }
}

export function isIpcContractValidationError(error: unknown): error is IpcContractValidationError {
  return error instanceof IpcContractValidationError;
}
