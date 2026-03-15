import { validatePluginManifest } from '../../utils/plugin-manifest.js';
import { validManifest } from './fixtures.js';

describe('validatePluginManifest prefetchOn', () => {
  it('accepts a string array when present', () => {
    const result = validatePluginManifest(
      validManifest({ prefetchOn: ['search-panel', 'file-explorer'] }),
    );

    expect(result.valid).toBe(true);
  });

  it('rejects non-array prefetchOn values', () => {
    const result = validatePluginManifest(validManifest({ prefetchOn: 'search-panel' }));

    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'prefetchOn')).toBe(true);
  });

  it('rejects empty prefetchOn entries', () => {
    const result = validatePluginManifest(validManifest({ prefetchOn: ['search-panel', ''] }));

    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'prefetchOn[1]')).toBe(true);
  });
});
