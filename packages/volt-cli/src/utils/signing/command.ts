import { execFileSync } from 'node:child_process';

interface CommandErrorShape {
  code?: string;
  signal?: NodeJS.Signals;
  status?: number;
  stderr?: string | Buffer;
  stdout?: string | Buffer;
}

export interface RunSigningCommandOptions {
  description?: string;
  echoStdout?: boolean;
}

export interface SigningCommandResult {
  stderr: string;
  stdout: string;
}

function normalizeOutput(value: string | Buffer | undefined): string {
  if (typeof value === 'string') {
    return value.trim();
  }
  if (Buffer.isBuffer(value)) {
    return value.toString('utf8').trim();
  }
  return '';
}

function writeOutput(stream: 'stderr' | 'stdout', value: string): void {
  if (!value) {
    return;
  }

  const content = value.endsWith('\n') ? value : `${value}\n`;
  if (stream === 'stdout') {
    process.stdout.write(content);
    return;
  }
  process.stderr.write(content);
}

/**
 * Run a signing command with captured output. If the command fails, include stdout/stderr in the thrown error.
 */
export function runSigningCommand(
  command: string,
  args: string[],
  options: RunSigningCommandOptions = {},
): SigningCommandResult {
  try {
    const stdout = execFileSync(command, args, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const normalizedStdout = normalizeOutput(stdout);
    if (options.echoStdout !== false) {
      writeOutput('stdout', normalizedStdout);
    }
    return {
      stdout: normalizedStdout,
      stderr: '',
    };
  } catch (error) {
    const commandError = (error ?? {}) as CommandErrorShape;
    const stdout = normalizeOutput(commandError.stdout);
    const stderr = normalizeOutput(commandError.stderr);
    const scope = options.description ?? command;
    const status = typeof commandError.status === 'number' ? `exit code ${commandError.status}` : undefined;
    const signal = typeof commandError.signal === 'string' ? `signal ${commandError.signal}` : undefined;
    const reason = commandError.code ? `reason ${commandError.code}` : undefined;
    const outcome = [status, signal, reason].filter(Boolean).join(', ');

    const lines = [`[volt] ${scope} failed${outcome ? ` (${outcome})` : ''}.`];
    if (stdout) {
      lines.push(`[volt] ${scope} stdout: ${stdout}`);
    }
    if (stderr) {
      lines.push(`[volt] ${scope} stderr: ${stderr}`);
    }

    throw new Error(lines.join('\n'));
  }
}
