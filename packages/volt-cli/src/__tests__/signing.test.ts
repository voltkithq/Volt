import { vi } from 'vitest';

vi.mock('node:child_process', () => ({
  execFileSync: vi.fn(),
}));

vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    existsSync: vi.fn(() => true),
    unlinkSync: vi.fn(),
    writeFileSync: vi.fn(),
    renameSync: vi.fn(),
  };
});

import './signing.resolve-config.suite.js';
import './signing.provider.suite.js';
import './signing.tool-availability.suite.js';
import './signing.macos.suite.js';
import './signing.windows.suite.js';
import './signing.providers.suite.js';
