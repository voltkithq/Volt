import { describe, expect, it } from 'vitest';
import { createPluginCommand } from '../../commands/plugin.js';

describe('plugin command registration', () => {
  it('registers the expected subcommands', () => {
    const command = createPluginCommand();
    expect(command.commands.map((child) => child.name())).toEqual([
      'init',
      'build',
      'test',
      'doctor',
    ]);
  });
});
