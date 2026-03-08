import { buildAliasLookup } from './aliases.js';
import { parseWithSchema, registerChannel } from './validation.js';
import type {
  ContractHandlers,
  InferCommandRequest,
  IpcCommandMap,
  IpcRegistrar,
} from './types.js';

export function registerContractHandlers<TCommands extends IpcCommandMap>(
  ipc: IpcRegistrar,
  commands: TCommands,
  handlers: ContractHandlers<TCommands>,
): void {
  const aliasLookup = buildAliasLookup(commands);
  const commandKeys = Object.keys(commands) as Array<keyof TCommands & string>;

  for (const commandKey of commandKeys) {
    const command = commands[commandKey];
    const handler = handlers[commandKey];
    if (typeof handler !== 'function') {
      throw new Error(`Missing handler for IPC command: ${commandKey}`);
    }

    const wrapped = async (rawArgs: unknown): Promise<unknown> => {
      const args = parseWithSchema(command.request, rawArgs, commandKey, 'request');
      const result = await handler(args as InferCommandRequest<typeof command>);
      return parseWithSchema(command.response, result, commandKey, 'response');
    };

    registerChannel(ipc, commandKey, wrapped);

    for (const [alias, canonical] of aliasLookup.entries()) {
      if (canonical !== commandKey || alias === commandKey) {
        continue;
      }
      registerChannel(ipc, alias, wrapped);
    }
  }
}
