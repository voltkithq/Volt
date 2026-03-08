import type { IpcCommandMap } from './types.js';

function normalizeAliases(aliases: readonly string[] | undefined): string[] {
  if (!aliases || aliases.length === 0) {
    return [];
  }

  const unique = new Set<string>();
  for (const alias of aliases) {
    const normalized = alias.trim();
    if (!normalized) {
      continue;
    }
    unique.add(normalized);
  }
  return [...unique];
}

export function buildAliasLookup<TCommands extends IpcCommandMap>(
  commands: TCommands,
): Map<string, string> {
  const lookup = new Map<string, string>();
  const commandKeys = Object.keys(commands) as Array<keyof TCommands & string>;

  for (const commandKey of commandKeys) {
    const existing = lookup.get(commandKey);
    if (existing && existing !== commandKey) {
      throw new Error(
        `IPC contract alias conflict: channel "${commandKey}" already mapped to "${existing}"`,
      );
    }
    lookup.set(commandKey, commandKey);

    for (const alias of normalizeAliases(commands[commandKey].aliases)) {
      const aliasOwner = lookup.get(alias);
      if (aliasOwner && aliasOwner !== commandKey) {
        throw new Error(
          `IPC contract alias conflict: alias "${alias}" is mapped to both "${aliasOwner}" and "${commandKey}"`,
        );
      }
      lookup.set(alias, commandKey);
    }
  }

  return lookup;
}

export function resolveContractChannel<TCommands extends IpcCommandMap>(
  commands: TCommands,
  channel: string,
): string {
  const aliasLookup = buildAliasLookup(commands);
  return aliasLookup.get(channel) ?? channel;
}
