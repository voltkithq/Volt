import { execFile } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ext = process.platform === 'win32' ? '.exe' : '';

export async function ensurePluginHostBinary(): Promise<string> {
  const configured = process.env.VOLT_PLUGIN_HOST_PATH;
  if (configured && existsSync(configured)) {
    return configured;
  }

  const repoRoot = resolveRepoRoot();
  const debugBinary = resolve(repoRoot, 'target', 'debug', `volt-plugin-host${ext}`);
  if (existsSync(debugBinary)) {
    return debugBinary;
  }

  await execCargoBuild(repoRoot);
  if (!existsSync(debugBinary)) {
    throw new Error(`[volt:plugin] Failed to locate built plugin host binary at ${debugBinary}`);
  }
  return debugBinary;
}

function resolveRepoRoot(): string {
  return resolve(fileURLToPath(new URL('../../../../', import.meta.url)));
}

function execCargoBuild(cwd: string): Promise<void> {
  return new Promise((resolvePromise, reject) => {
    execFile(
      'cargo',
      ['build', '-p', 'volt-plugin-host'],
      { cwd },
      (error, stdout, stderr) => {
        if (error) {
          reject(
            new Error(
              `[volt:plugin] Failed to build volt-plugin-host: ${stderr || stdout || error.message}`,
            ),
          );
          return;
        }
        resolvePromise();
      },
    );
  });
}
