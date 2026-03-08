import { execFile } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { promisify } from 'node:util';
import { sanitizePathSegment } from './path.js';
import type { VoltTestLogger } from './types.js';

const execFileAsync = promisify(execFile);

export interface ScreenshotCommand {
  command: string;
  args: string[];
}

export function createRunArtifactRoot(repoRoot: string, artifactDir?: string): string {
  const root = artifactDir
    ? resolve(repoRoot, artifactDir)
    : resolve(repoRoot, 'artifacts', 'volt-test', timestampForPath(new Date()));
  mkdirSync(root, { recursive: true });
  return root;
}

export function createSuiteAttemptArtifactDir(
  runArtifactRoot: string,
  suiteName: string,
  attemptNumber: number,
): string {
  const suiteSegment = sanitizePathSegment(suiteName);
  const attemptSegment = `attempt-${attemptNumber}`;
  const suiteDir = join(runArtifactRoot, suiteSegment, attemptSegment);
  mkdirSync(suiteDir, { recursive: true });
  return suiteDir;
}

export function writeJsonArtifact(filePath: string, payload: unknown): void {
  const serialized = `${JSON.stringify(payload, null, 2)}\n`;
  writeTextArtifact(filePath, serialized);
}

export function writeTextArtifact(filePath: string, contents: string): void {
  mkdirSync(dirname(filePath), { recursive: true });
  writeFileSync(filePath, contents, 'utf8');
}

export async function captureDesktopScreenshot(
  screenshotPath: string,
  logger: VoltTestLogger,
  platform = process.platform,
): Promise<boolean> {
  const screenshotCommand = buildScreenshotCommand(platform, screenshotPath);
  if (!screenshotCommand) {
    logger.warn(`[volt:test] screenshot capture is not supported on ${platform}.`);
    return false;
  }

  try {
    await execFileAsync(screenshotCommand.command, screenshotCommand.args, {
      timeout: 15_000,
      windowsHide: true,
    });
    logger.log(`[volt:test] screenshot captured: ${screenshotPath}`);
    return true;
  } catch (error) {
    logger.warn(
      `[volt:test] failed to capture screenshot at ${screenshotPath}: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
    return false;
  }
}

export function buildScreenshotCommand(platform: string, screenshotPath: string): ScreenshotCommand | null {
  if (platform === 'darwin') {
    return {
      command: 'screencapture',
      args: ['-x', screenshotPath],
    };
  }

  if (platform === 'win32') {
    const captureScript = [
      'Add-Type -AssemblyName System.Windows.Forms',
      'Add-Type -AssemblyName System.Drawing',
      '$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen',
      '$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height',
      '$graphics = [System.Drawing.Graphics]::FromImage($bitmap)',
      '$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)',
      '$bitmap.Save($args[0], [System.Drawing.Imaging.ImageFormat]::Png)',
      '$graphics.Dispose()',
      '$bitmap.Dispose()',
    ].join('; ');

    return {
      command: 'powershell',
      args: [
        '-NoProfile',
        '-NonInteractive',
        '-ExecutionPolicy',
        'Bypass',
        '-Command',
        captureScript,
        screenshotPath,
      ],
    };
  }

  if (platform === 'linux') {
    const script = [
      'if command -v import >/dev/null 2>&1; then',
      '  import -window root "$1"',
      'elif command -v gnome-screenshot >/dev/null 2>&1; then',
      '  gnome-screenshot -f "$1"',
      'else',
      '  exit 127',
      'fi',
    ].join('; ');

    return {
      command: 'sh',
      args: ['-lc', script, 'volt-test-screenshot', screenshotPath],
    };
  }

  return null;
}

function timestampForPath(date: Date): string {
  return date.toISOString()
    .replace(/[:]/g, '-')
    .replace(/\..+$/, 'Z');
}

export const __testOnly = {
  sanitizePathSegment,
  timestampForPath,
};
