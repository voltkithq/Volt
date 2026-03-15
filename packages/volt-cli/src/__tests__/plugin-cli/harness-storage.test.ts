import { describe, expect, it } from 'vitest';
import { performStorageRequest } from '../../utils/plugin-host-harness/storage.js';
import { createTempWorkspace } from './fixtures.js';

describe('plugin harness storage', () => {
  it('stores, lists, and deletes values', () => {
    const dataRoot = createTempWorkspace();

    performStorageRequest(dataRoot, 'set', { key: 'alpha', value: 'one' });
    performStorageRequest(dataRoot, 'set', { key: 'beta', value: 'two' });

    expect(performStorageRequest(dataRoot, 'get', { key: 'alpha' })).toBe('one');
    expect(performStorageRequest(dataRoot, 'has', { key: 'beta' })).toBe(true);
    expect(performStorageRequest(dataRoot, 'keys', null)).toEqual(['alpha', 'beta']);

    performStorageRequest(dataRoot, 'delete', { key: 'alpha' });

    expect(performStorageRequest(dataRoot, 'get', { key: 'alpha' })).toBeNull();
    expect(performStorageRequest(dataRoot, 'keys', null)).toEqual(['beta']);
  });

  it('rejects invalid keys and oversized values', () => {
    const dataRoot = createTempWorkspace();

    expect(() =>
      performStorageRequest(dataRoot, 'set', { key: '../escape', value: 'x' }),
    ).toThrow(/invalid storage key/);
    expect(() =>
      performStorageRequest(dataRoot, 'set', {
        key: 'large',
        value: 'x'.repeat(1024 * 1024 + 1),
      }),
    ).toThrow(/storage value exceeds/);
  });
});
