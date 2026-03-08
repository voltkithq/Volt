import { buildAliasLookup } from './aliases.js';
import type { IpcCommandMap } from './types.js';

export function defineCommands<const TCommands extends IpcCommandMap>(
  commands: TCommands,
): TCommands {
  // Precompute alias resolution early so conflicts fail at definition time.
  buildAliasLookup(commands);
  return commands;
}
