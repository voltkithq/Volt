export function validManifest(overrides?: Record<string, unknown>) {
  return {
    id: 'acme.notes.search',
    name: 'Notes Search',
    version: '0.1.0',
    apiVersion: 1,
    engine: { volt: '>=0.2.0' },
    backend: './dist/plugin.js',
    capabilities: ['http', 'fs'],
    ...overrides,
  };
}
