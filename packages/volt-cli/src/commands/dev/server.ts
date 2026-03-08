import { createRequire } from 'node:module';
import { spawn, type ChildProcess } from 'node:child_process';
import { stripVTControlCharacters } from 'node:util';

const VITE_URL_PATTERN = /https?:\/\/[^\s]+/i;

export interface SpawnedVite {
  child: ChildProcess;
  detectedUrl: Promise<string | null>;
}

export function parseDevPort(raw: string): number {
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
    throw new Error(`Invalid --port value "${raw}". Expected an integer in range 1-65535.`);
  }
  return parsed;
}

/**
 * Spawn Vite as a child process using Node.js directly.
 * We resolve vite's CLI bin from this package's context (volt-cli depends on vite).
 */
export function spawnVite(cwd: string, port: number, host: string): SpawnedVite {
  const require = createRequire(import.meta.url);
  let viteBin: string;
  try {
    viteBin = require.resolve('vite/bin/vite.js');
  } catch {
    const vitePkg = require.resolve('vite/package.json');
    viteBin = vitePkg.replace('package.json', 'bin/vite.js');
  }

  const child = spawn(process.execPath, [viteBin, '--port', String(port), '--host', host], {
    cwd,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  const detector = createViteUrlDetector(child);
  pipeViteOutput(child.stdout, process.stdout, detector.onLine);
  pipeViteOutput(child.stderr, process.stderr, detector.onLine);

  child.on('error', (err) => {
    console.error(`[volt] Failed to start Vite: ${err.message}`);
  });

  child.on('exit', (code) => {
    if (code !== 0 && code !== null) {
      console.error(`[volt] Vite exited with code ${code}`);
    }
  });

  return {
    child,
    detectedUrl: detector.detectedUrl,
  };
}

/**
 * Poll the server URL until it responds.
 */
export async function waitForServer(url: string, timeoutMs: number): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url, { signal: AbortSignal.timeout(1000) });
      if (res.ok) {
        return true;
      }
    } catch {
      // not ready yet
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  return false;
}

export async function resolveViteDevUrl(
  vite: SpawnedVite,
  fallbackUrl: string,
  timeoutMs: number,
): Promise<string | null> {
  const deadlineMs = Date.now() + timeoutMs;
  const detectedUrl = await waitForDetectedViteUrl(vite, timeoutMs);
  const candidates: string[] = [];
  if (detectedUrl) {
    const normalizedDetectedUrl = normalizeDevServerUrl(detectedUrl);
    if (normalizedDetectedUrl.length > 0) {
      candidates.push(normalizedDetectedUrl);
    }
  }

  const normalizedFallbackUrl = normalizeDevServerUrl(fallbackUrl);
  if (normalizedFallbackUrl.length > 0 && !candidates.includes(normalizedFallbackUrl)) {
    candidates.push(normalizedFallbackUrl);
  }

  for (const candidateUrl of candidates) {
    const remainingMs = deadlineMs - Date.now();
    if (remainingMs <= 0) {
      return null;
    }
    const ready = await waitForServer(candidateUrl, Math.max(250, remainingMs));
    if (ready) {
      return candidateUrl;
    }
  }

  return null;
}

export function extractViteLocalUrl(outputLine: string): string | null {
  const line = stripAnsi(outputLine).trim();
  if (!/\blocal\b/i.test(line)) {
    return null;
  }

  const match = line.match(VITE_URL_PATTERN);
  if (!match) {
    return null;
  }

  return normalizeDevServerUrl(match[0]);
}

function waitForDetectedViteUrl(vite: SpawnedVite, timeoutMs: number): Promise<string | null> {
  const timeoutPromise = new Promise<null>((resolve) => {
    const timer = setTimeout(() => resolve(null), timeoutMs);
    if (typeof (timer as { unref?: () => void }).unref === 'function') {
      (timer as { unref: () => void }).unref();
    }
  });
  const childExitPromise = new Promise<null>((resolve) => {
    vite.child.once('exit', () => resolve(null));
  });

  return Promise.race([vite.detectedUrl, childExitPromise, timeoutPromise]);
}

function pipeViteOutput(
  stream: NodeJS.ReadableStream | null,
  target: NodeJS.WritableStream,
  onLine: (line: string) => void,
): void {
  if (!stream) {
    return;
  }

  let buffered = '';
  stream.on('data', (chunk: Buffer | string) => {
    const text = chunk.toString();
    target.write(text);
    buffered += text;

    let normalized = buffered.replace(/\r\n/g, '\n').replace(/\r/g, '\n');
    let newline = normalized.indexOf('\n');
    while (newline >= 0) {
      const line = normalized.slice(0, newline);
      onLine(line);
      normalized = normalized.slice(newline + 1);
      newline = normalized.indexOf('\n');
    }
    buffered = normalized;
  });

  stream.on('end', () => {
    if (buffered.length > 0) {
      onLine(buffered);
    }
  });
}

function createViteUrlDetector(child: ChildProcess): {
  detectedUrl: Promise<string | null>;
  onLine: (line: string) => void;
} {
  let settled = false;
  let resolveDetectedUrl: (url: string | null) => void = () => {};

  const detectedUrl = new Promise<string | null>((resolve) => {
    resolveDetectedUrl = resolve;
  });

  const settle = (url: string | null): void => {
    if (settled) {
      return;
    }
    settled = true;
    resolveDetectedUrl(url);
  };

  child.once('error', () => settle(null));
  child.once('exit', () => settle(null));

  return {
    detectedUrl,
    onLine: (line: string) => {
      if (settled) {
        return;
      }
      const maybeUrl = extractViteLocalUrl(line);
      if (maybeUrl) {
        settle(maybeUrl);
      }
    },
  };
}

function stripAnsi(value: string): string {
  return stripVTControlCharacters(value);
}

function normalizeDevServerUrl(url: string): string {
  const withoutPunctuation = url.replace(/[),;]+$/, '');
  return withoutPunctuation.replace(/\/+$/, '');
}
