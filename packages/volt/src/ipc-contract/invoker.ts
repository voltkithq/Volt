import { buildAliasLookup } from './aliases.js';
import { parseWithSchema } from './validation.js';
import type {
  InferCommandRequest,
  InferCommandResponse,
  IpcCommandMap,
  IpcInvokeFn,
  TypedIpcInvoker,
} from './types.js';

export function createLegacyInvokeAdapter<TCommands extends IpcCommandMap>(
  commands: TCommands,
  invokeFn: IpcInvokeFn,
): (channel: string, args: unknown) => Promise<unknown> {
  const aliasLookup = buildAliasLookup(commands);
  return async (channel: string, args: unknown): Promise<unknown> => {
    const resolvedChannel = aliasLookup.get(channel) ?? channel;
    return invokeFn(resolvedChannel, args);
  };
}

export function createContractInvoker<TCommands extends IpcCommandMap>(
  commands: TCommands,
  invokeFn: IpcInvokeFn,
): TypedIpcInvoker<TCommands> {
  const aliasLookup = buildAliasLookup(commands);

  const resolveChannel = (channel: string): string => aliasLookup.get(channel) ?? channel;

  const invokeLegacy = async (channel: string, args: unknown): Promise<unknown> => {
    const resolvedChannel = resolveChannel(channel);
    return invokeFn(resolvedChannel, args);
  };

  return {
    resolveChannel,
    invokeLegacy,
    async invoke<TKey extends keyof TCommands & string>(
      channel: TKey,
      args: InferCommandRequest<TCommands[TKey]>,
    ): Promise<InferCommandResponse<TCommands[TKey]>> {
      const resolvedChannel = resolveChannel(channel);
      if (!Object.prototype.hasOwnProperty.call(commands, resolvedChannel)) {
        throw new Error(`Unknown IPC contract channel: ${String(channel)}`);
      }

      const command = commands[resolvedChannel as keyof TCommands];
      const parsedArgs = parseWithSchema(command.request, args, resolvedChannel, 'request');
      const rawResult = await invokeFn(resolvedChannel, parsedArgs);
      return parseWithSchema(
        command.response,
        rawResult,
        resolvedChannel,
        'response',
      ) as InferCommandResponse<TCommands[TKey]>;
    },
  };
}
