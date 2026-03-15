import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../../__mocks__/volt-native.js');
});

import { fs, setBaseDir } from '../../fs.js';
import { fsWatchClose, fsWatchPoll, fsWatchStart } from '@voltkit/volt-native';

describe('fs watchers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setBaseDir('/mock/base');
  });

  it('creates a watcher and returns a FileWatcher handle', async () => {
    const watcher = await fs.watch('data');
    expect(fsWatchStart).toHaveBeenCalledWith('/mock/base', 'data', true, 200);
    expect(typeof watcher.poll).toBe('function');
    expect(typeof watcher.close).toBe('function');
  });

  it('passes custom watch options', async () => {
    await fs.watch('logs', { recursive: false, debounceMs: 500 });
    expect(fsWatchStart).toHaveBeenCalledWith('/mock/base', 'logs', false, 500);
  });

  it('poll drains native watcher events', async () => {
    const watcher = await fs.watch('data');
    const events = await watcher.poll();
    expect(fsWatchPoll).toHaveBeenCalled();
    expect(events).toEqual([]);
  });

  it('close releases the native watcher', async () => {
    const watcher = await fs.watch('data');
    await watcher.close();
    expect(fsWatchClose).toHaveBeenCalled();
  });

  it('rejects invalid watch paths', async () => {
    await expect(fs.watch('/etc')).rejects.toThrow('Absolute paths');
    await expect(fs.watch('../../secret')).rejects.toThrow('Path traversal');
  });
});

describe('scoped watchers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setBaseDir('/mock/base');
  });

  it('creates a scoped watcher via ScopedFs.watch', async () => {
    const scopedFs = await fs.bindScope('test_grant_watch');
    const watcher = await scopedFs.watch('subdir');
    expect(fsWatchStart).toHaveBeenCalledWith('/mock/grant/path', 'subdir', true, 200);
    expect(watcher).toBeDefined();
  });

  it('allows the scope root watcher path', async () => {
    const scopedFs = await fs.bindScope('test_grant_watch_root');
    await scopedFs.watch('');
    expect(fsWatchStart).toHaveBeenCalledWith('/mock/grant/path', '', true, 200);
  });

  it('rejects invalid scoped watcher paths', async () => {
    const scopedFs = await fs.bindScope('test_grant_watch_checks');
    await expect(scopedFs.watch('../../secret')).rejects.toThrow('Path traversal');
    await expect(scopedFs.watch('/etc')).rejects.toThrow('Absolute paths');
  });
});
