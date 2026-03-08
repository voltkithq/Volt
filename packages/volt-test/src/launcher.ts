import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { createWriteStream, existsSync, mkdirSync, readFileSync, type WriteStream } from 'node:fs';
import { join, resolve } from 'node:path';
import { captureDesktopScreenshot, writeJsonArtifact } from './artifacts.js';
import { cleanupDirectoryBestEffort, copyProjectToTemp } from './fs.js';
import { readJsonFileWithRetry, terminateChildProcess, waitForChildExit, waitForFile } from './process.js';
import type { VoltTestLogger } from './types.js';

const DEFAULT_LAUNCH_TIMEOUT_MS = 120_000;
const DEFAULT_PROCESS_EXIT_TIMEOUT_MS = 30_000;

interface RuntimeArtifactManifest {
  artifactFileName: string;
}

export interface VoltAppLauncherOptions {
  repoRoot: string;
  cliEntryPath: string;
  logger: VoltTestLogger;
}

export interface RunScenarioOptions<TPayload> {
  sourceProjectDir: string;
  resultFile: string;
  timeoutMs?: number;
  prepareProject?: (projectDir: string) => Promise<void> | void;
  validatePayload?: (payload: unknown) => TPayload;
  preserveTempDir?: boolean;
  artifactsDir?: string;
  logFileName?: string;
  screenshotFileName?: string;
  captureScreenshotOnError?: boolean;
}

/**
 * Runs a Volt app scenario in an isolated temp project copy.
 * The launcher builds the copied project, launches the produced runtime artifact,
 * waits for a JSON result file, validates the payload, and enforces process cleanup.
 */
export class VoltAppLauncher {
  private readonly repoRoot: string;
  private readonly cliEntryPath: string;
  private readonly logger: VoltTestLogger;

  public constructor(options: VoltAppLauncherOptions) {
    this.repoRoot = resolve(options.repoRoot);
    this.cliEntryPath = resolve(options.cliEntryPath);
    this.logger = options.logger;
  }

  /**
   * Execute one scenario and return its validated payload.
   * Set `preserveTempDir` when debugging to inspect generated temp artifacts.
   */
  public async run<TPayload = unknown>(options: RunScenarioOptions<TPayload>): Promise<TPayload> {
    this.ensureCliEntry();

    const sourceProjectDir = resolve(this.repoRoot, options.sourceProjectDir);
    const timeoutMs = options.timeoutMs ?? DEFAULT_LAUNCH_TIMEOUT_MS;
    const captureScreenshotOnError = options.captureScreenshotOnError ?? true;

    const copied = copyProjectToTemp(sourceProjectDir, this.repoRoot);
    const resultPath = join(copied.tempProjectDir, options.resultFile);
    const artifactsDir = options.artifactsDir ? resolve(options.artifactsDir) : null;
    const processLogPath = artifactsDir ? join(artifactsDir, options.logFileName ?? 'app-process.log') : null;
    const screenshotPath = artifactsDir ? join(artifactsDir, options.screenshotFileName ?? 'failure.png') : null;
    let logStream: WriteStream | null = null;
    let child: ChildProcess | null = null;

    if (artifactsDir) {
      mkdirSync(artifactsDir, { recursive: true });
      if (processLogPath) {
        logStream = createWriteStream(processLogPath, { flags: 'a' });
      }
    }

    try {
      if (options.prepareProject) {
        await options.prepareProject(copied.tempProjectDir);
      }

      this.logger.log(`[volt:test] building ${options.sourceProjectDir}`);
      this.buildProject(copied.tempProjectDir);

      const runtimeBinaryPath = resolveRuntimeBinary(copied.tempProjectDir);
      this.logger.log(`[volt:test] launching ${runtimeBinaryPath}`);
      child = this.launchBinary(runtimeBinaryPath, copied.tempProjectDir);
      attachChildOutput(child, logStream);

      const resultReady = await waitForFile(resultPath, timeoutMs);
      if (!resultReady) {
        if (child) {
          await terminateChildProcess(
            child,
            `timeout waiting for ${options.resultFile}`,
            this.logger,
          );
        }
        throw new Error(
          `[volt:test] timed out waiting for result file "${options.resultFile}" from ${options.sourceProjectDir}`,
        );
      }

      const payload = await readJsonFileWithRetry<unknown>(resultPath, 2_000);
      const validatedPayload = options.validatePayload
        ? options.validatePayload(payload)
        : (payload as TPayload);

      if (artifactsDir) {
        writeJsonArtifact(join(artifactsDir, 'result-payload.json'), payload);
      }

      if (child) {
        const exitResult = await waitForChildExit(child, DEFAULT_PROCESS_EXIT_TIMEOUT_MS);
        if (!exitResult) {
          await terminateChildProcess(child, 'app did not exit after result write', this.logger);
          throw new Error(
            `[volt:test] app for ${options.sourceProjectDir} did not exit after reporting completion`,
          );
        }
        if (exitResult.code !== 0) {
          throw new Error(
            `[volt:test] app for ${options.sourceProjectDir} exited with code ${exitResult.code} (signal: ${
              exitResult.signal ?? 'none'
            })`,
          );
        }
      }

      return validatedPayload;
    } catch (error) {
      if (captureScreenshotOnError && screenshotPath) {
        await captureDesktopScreenshot(screenshotPath, this.logger);
      }
      throw error;
    } finally {
      if (child && (child.exitCode === null && child.signalCode === null)) {
        await terminateChildProcess(child, 'suite cleanup', this.logger);
      }

      if (logStream) {
        await closeWriteStream(logStream);
      }

      if (!options.preserveTempDir) {
        await cleanupDirectoryBestEffort(copied.tempRoot, this.logger);
      } else {
        this.logger.log(`[volt:test] preserved temp directory: ${copied.tempRoot}`);
      }
    }
  }

  private ensureCliEntry(): void {
    if (!existsSync(this.cliEntryPath)) {
      throw new Error(
        `[volt:test] volt CLI entry not found: ${this.cliEntryPath}. Build @voltkit/volt-cli first.`,
      );
    }
  }

  private buildProject(projectDir: string): void {
    execFileSync('node', [this.cliEntryPath, 'build'], {
      cwd: projectDir,
      stdio: 'inherit',
      env: process.env,
    });
  }

  private launchBinary(binaryPath: string, cwd: string): ChildProcess {
    if (process.platform === 'linux') {
      return spawn('xvfb-run', ['-a', binaryPath], {
        cwd,
        stdio: ['ignore', 'pipe', 'pipe'],
        env: process.env,
      });
    }

    return spawn(binaryPath, [], {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: process.env,
    });
  }
}

function resolveRuntimeBinary(projectDir: string): string {
  const manifestPath = join(projectDir, 'dist-volt', '.volt-runtime-artifact.json');
  if (!existsSync(manifestPath)) {
    throw new Error(`[volt:test] runtime manifest missing: ${manifestPath}`);
  }

  const manifestRaw = readFileSync(manifestPath, 'utf8');
  const manifest = JSON.parse(manifestRaw) as RuntimeArtifactManifest;
  if (!manifest || typeof manifest.artifactFileName !== 'string' || manifest.artifactFileName.length === 0) {
    throw new Error(`[volt:test] invalid runtime manifest at ${manifestPath}`);
  }

  const binaryPath = join(projectDir, 'dist-volt', manifest.artifactFileName);
  if (!existsSync(binaryPath)) {
    throw new Error(`[volt:test] runtime binary missing: ${binaryPath}`);
  }
  return binaryPath;
}

function attachChildOutput(child: ChildProcess, logStream: WriteStream | null): void {
  if (child.stdout) {
    child.stdout.on('data', (chunk: Buffer | string) => {
      process.stdout.write(chunk);
      if (logStream) {
        logStream.write(chunk);
      }
    });
  }

  if (child.stderr) {
    child.stderr.on('data', (chunk: Buffer | string) => {
      process.stderr.write(chunk);
      if (logStream) {
        logStream.write(chunk);
      }
    });
  }
}

async function closeWriteStream(stream: WriteStream): Promise<void> {
  await new Promise<void>((resolveStream, reject) => {
    stream.end(() => resolveStream());
    stream.once('error', (error) => reject(error));
  });
}

export const __testOnly = {
  resolveRuntimeBinary,
};
