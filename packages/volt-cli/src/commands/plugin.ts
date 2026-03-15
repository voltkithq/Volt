import { Command } from 'commander';
import { pluginBuildCommand } from './plugin/build.js';
import { pluginDoctorCommand } from './plugin/doctor.js';
import { pluginInitCommand } from './plugin/init.js';
import { pluginTestCommand } from './plugin/test.js';

export function createPluginCommand(): Command {
  const command = new Command('plugin').description('Plugin development tooling');

  command
    .command('init [name]')
    .description('Scaffold a new Volt plugin project')
    .action(async (name) => {
      await pluginInitCommand(name);
    });

  command
    .command('build')
    .description('Bundle a Volt plugin backend for production')
    .action(async () => {
      await pluginBuildCommand();
    });

  command
    .command('test')
    .description('Smoke-test a plugin in the real plugin host')
    .action(async () => {
      await pluginTestCommand();
    });

  command
    .command('doctor')
    .description('Validate plugin manifest, entrypoints, and host compatibility')
    .action(async () => {
      await pluginDoctorCommand();
    });

  return command;
}
