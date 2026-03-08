#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { Command } from 'commander';
import { devCommand } from './commands/dev.js';
import { buildCommand } from './commands/build.js';
import { previewCommand } from './commands/preview.js';
import { packageCommand } from './commands/package.js';
import { doctorCommand } from './commands/doctor.js';
import { signSetupCommand } from './commands/sign.js';
import { testCommand } from './commands/test.js';
import { updatePublishCommand } from './commands/update.js';

function resolveCliVersion(): string {
  try {
    const packageJsonPath = new URL('../package.json', import.meta.url);
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as { version?: string };
    return packageJson.version ?? '0.1.0';
  } catch {
    return '0.1.0';
  }
}

const program = new Command();

function collectOptionValues(value: string, previous: string[]): string[] {
  return [...previous, value];
}

program
  .name('volt')
  .description('Volt - Lightweight Desktop App Framework')
  .version(resolveCliVersion());

program
  .command('dev')
  .description('Start development server with hot reload')
  .option('-p, --port <port>', 'Vite dev server port', '5173')
  .option('--host <host>', 'Vite dev server host', 'localhost')
  .action(async (options) => {
    await devCommand(options);
  });

program
  .command('build')
  .description('Build the application for production')
  .option('--target <target>', 'Rust target triple for cross-compilation')
  .action(async (options) => {
    await buildCommand(options);
  });

program
  .command('preview')
  .description('Preview the production build locally')
  .action(async () => {
    await previewCommand();
  });

program
  .command('test')
  .description('Run Volt end-to-end smoke/integration suites')
  .option('--config <path>', 'Path to Volt test configuration file')
  .option('--suite <name>', 'Run only specific suite(s). Repeat for multiple suites.', collectOptionValues, [])
  .option('--list', 'List configured suites without running them')
  .option('--timeout <ms>', 'Override suite timeout in milliseconds')
  .option('--retries <count>', 'Retry failing suites up to N additional attempts')
  .option('--artifacts-dir <path>', 'Directory for test artifacts (logs/screenshots/summaries)')
  .action(async (options) => {
    await testCommand(options);
  });

program
  .command('doctor')
  .description('Validate packaging and signing prerequisites')
  .option('--target <target>', 'Target platform (win32, darwin, linux)')
  .option('--format <format>', 'Package format (nsis, msix, app, dmg, appimage, deb)')
  .option('--json', 'Print machine-readable doctor report JSON')
  .action(async (options) => {
    await doctorCommand(options);
  });

program
  .command('package')
  .description('Package the application into platform-specific installers')
  .option('--target <target>', 'Target platform (win32, darwin, linux)')
  .option('--format <format>', 'Package format (nsis, msix, app, dmg, appimage, deb)')
  .option('--install-mode <mode>', 'Windows install mode (perMachine, perUser)')
  .option('--json', 'Print machine-readable package summary JSON')
  .option('--json-output <path>', 'Write machine-readable package summary JSON to file')
  .action(async (options) => {
    await packageCommand(options);
  });

const signProgram = program
  .command('sign')
  .description('Signing setup and tooling helpers');

signProgram
  .command('setup')
  .description('Generate signing environment template and prerequisite checks')
  .option('--platform <platform>', 'Target platform (darwin, win32, all)')
  .option(
    '--windows-provider <provider>',
    'Windows provider (local, azureTrustedSigning, digicertKeyLocker)',
  )
  .option('--output <path>', 'Output path for generated env template', '.env.signing')
  .option('--force', 'Overwrite output file if it already exists')
  .option('--print', 'Print generated template to stdout')
  .option('--print-only', 'Print template only (do not write output file)')
  .action(async (options) => {
    await signSetupCommand(options);
  });

const updateProgram = program
  .command('update')
  .description('Manage update publishing artifacts');

updateProgram
  .command('publish')
  .description('Generate and publish update artifacts + manifest')
  .option('--artifacts-dir <dir>', 'Directory containing built runtime artifacts', 'dist-volt')
  .option('--out-dir <dir>', 'Publish output directory', 'dist-update')
  .option('--provider <provider>', 'Publish provider (local)', 'local')
  .option('--channel <channel>', 'Release channel', 'stable')
  .option('--base-url <url>', 'Base URL used in generated manifest artifact URLs')
  .option('--manifest-file <name>', 'Manifest file name')
  .option('--dry-run', 'Run preflight + manifest generation without writing files')
  .action(async (options) => {
    await updatePublishCommand(options);
  });

program.parse();
