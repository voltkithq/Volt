import { existsSync, readFileSync } from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  __testOnly,
  buildScreenshotCommand,
  createRunArtifactRoot,
  createSuiteAttemptArtifactDir,
  writeJsonArtifact,
} from './artifacts.js';

describe('artifact helpers', () => {
  it('creates deterministic run and suite artifact directories', () => {
    const repoRoot = mkdtempSync(join(tmpdir(), 'volt-test-artifacts-'));
    const root = createRunArtifactRoot(repoRoot, 'artifacts/custom-run');
    const suiteDir = createSuiteAttemptArtifactDir(root, 'ipc demo smoke', 2);

    expect(root).toContain(join('artifacts', 'custom-run'));
    expect(suiteDir).toContain(join('ipc-demo-smoke', 'attempt-2'));
  });

  it('writes json artifacts to disk', () => {
    const repoRoot = mkdtempSync(join(tmpdir(), 'volt-test-artifacts-json-'));
    const artifactPath = join(repoRoot, 'out', 'result.json');
    writeJsonArtifact(artifactPath, { ok: true, value: 42 });

    expect(existsSync(artifactPath)).toBe(true);
    expect(readFileSync(artifactPath, 'utf8')).toContain('"ok": true');
  });

  it('builds platform-specific screenshot commands', () => {
    expect(buildScreenshotCommand('darwin', '/tmp/out.png')).toEqual({
      command: 'screencapture',
      args: ['-x', '/tmp/out.png'],
    });

    const windows = buildScreenshotCommand('win32', 'C:\\temp\\out.png');
    expect(windows?.command).toBe('powershell');
    const scriptArg = windows?.args?.find((a) => a.includes('CopyFromScreen'));
    expect(scriptArg).toContain('C:\\temp\\out.png');

    const linux = buildScreenshotCommand('linux', '/tmp/out.png');
    expect(linux?.command).toBe('sh');
    expect(linux?.args[0]).toBe('-lc');
  });

  it('sanitizes suite names for file-system paths', () => {
    expect(__testOnly.sanitizePathSegment('  demo suite / smoke  ')).toBe('demo-suite-smoke');
  });
});
