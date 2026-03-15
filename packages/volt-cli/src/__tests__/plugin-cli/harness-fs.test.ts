import { mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { performScopedFsRequest } from '../../utils/plugin-host-harness/fs.js';
import { createTempWorkspace } from './fixtures.js';

describe('plugin harness fs', () => {
  it('reads and writes inside the scoped data root', () => {
    const dataRoot = createTempWorkspace();

    performScopedFsRequest(dataRoot, 'write-file', {
      path: 'notes/output.txt',
      data: 'hello from plugin',
    });

    expect(readFileSync(resolve(dataRoot, 'notes', 'output.txt'), 'utf8')).toBe('hello from plugin');
    expect(
      performScopedFsRequest(dataRoot, 'read-file', { path: 'notes/output.txt' }),
    ).toBe('hello from plugin');
  });

  it('rejects traversal outside the scoped data root', () => {
    const dataRoot = createTempWorkspace();
    mkdirSync(resolve(dataRoot, 'safe'), { recursive: true });

    expect(() =>
      performScopedFsRequest(dataRoot, 'read-file', { path: '../outside.txt' }),
    ).toThrow(/path escapes base directory|path traversal is not allowed/);
  });
});
