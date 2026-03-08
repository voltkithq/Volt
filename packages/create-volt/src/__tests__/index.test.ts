import { mkdtempSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { __testOnly } from '../index.js';

const tempDirs: string[] = [];

function createTempDir(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    if (dir) {
      rmSync(dir, { recursive: true, force: true });
    }
  }
});

describe('create-volt helpers', () => {
  it('normalizes valid project names', () => {
    expect(__testOnly.normalizeProjectName('My App')).toBe('my-app');
    expect(__testOnly.normalizeProjectName('demo_app')).toBe('demo_app');
    expect(__testOnly.normalizeProjectName('demo.app')).toBe('demo.app');
  });

  it('rejects invalid project names', () => {
    expect(() => __testOnly.normalizeProjectName('')).toThrow('cannot be empty');
    expect(() => __testOnly.normalizeProjectName('.')).toThrow('cannot be "." or ".."');
    expect(() => __testOnly.normalizeProjectName('../bad')).toThrow('single directory segment');
    expect(() => __testOnly.normalizeProjectName('Bad*Name')).toThrow('must be lowercase');
  });

  it('builds human-friendly display names', () => {
    expect(__testOnly.toDisplayName('my-volt-app')).toBe('My Volt App');
    expect(__testOnly.toDisplayName('todo_app')).toBe('Todo App');
  });

  it('escapes html entities in title content', () => {
    expect(__testOnly.escapeHtml('<Volt & App>')).toBe('&lt;Volt &amp; App&gt;');
  });

  it('updates only top-level name and window.title fields in volt config templates', () => {
    const input = `
import { defineConfig } from 'voltkit';

const metadata = {
  name: 'metadata-name-should-stay',
  title: 'metadata-title-should-stay',
};

export default defineConfig({
  name: 'template-name',
  version: '0.1.0',
  window: {
    title: 'template-title',
    width: 800,
    height: 600,
  },
  menu: {
    title: 'menu-title-should-stay',
  },
});
`.trim();

    const updated = __testOnly.updateVoltConfigContent(input, 'My Demo App');

    expect(updated).toContain(`name: "My Demo App"`);
    expect(updated).toContain(`title: "My Demo App"`);
    expect(updated).toContain(`name: 'metadata-name-should-stay'`);
    expect(updated).toContain(`title: 'metadata-title-should-stay'`);
    expect(updated).toContain(`title: 'menu-title-should-stay'`);
  });

  it('creates a project from templates and rewrites key files', async () => {
    const workspaceRoot = createTempDir('create-volt-workspace-');
    const previousCwd = process.cwd();
    process.chdir(workspaceRoot);

    try {
      await __testOnly.createProject({
        name: 'my-demo-app',
        displayName: 'My Demo App',
        framework: 'vanilla',
      });
    } finally {
      process.chdir(previousCwd);
    }

    const targetDir = resolve(workspaceRoot, 'my-demo-app');
    const packageJson = JSON.parse(readFileSync(resolve(targetDir, 'package.json'), 'utf8')) as { name?: string };
    const config = readFileSync(resolve(targetDir, 'volt.config.ts'), 'utf8');
    const html = readFileSync(resolve(targetDir, 'index.html'), 'utf8');

    expect(packageJson.name).toBe('my-demo-app');
    expect(config).toContain('name: "My Demo App"');
    expect(config).toContain('title: "My Demo App"');
    expect(html).toContain('<title>My Demo App</title>');
  });

  it('creates an enterprise starter with enterprise packaging defaults', async () => {
    const workspaceRoot = createTempDir('create-volt-enterprise-workspace-');
    const previousCwd = process.cwd();
    process.chdir(workspaceRoot);

    try {
      await __testOnly.createProject({
        name: 'enterprise-demo',
        displayName: 'Enterprise Demo',
        framework: 'enterprise',
      });
    } finally {
      process.chdir(previousCwd);
    }

    const targetDir = resolve(workspaceRoot, 'enterprise-demo');
    const packageJson = JSON.parse(readFileSync(resolve(targetDir, 'package.json'), 'utf8')) as {
      name?: string;
      scripts?: Record<string, string>;
    };
    const config = readFileSync(resolve(targetDir, 'volt.config.ts'), 'utf8');

    expect(packageJson.name).toBe('enterprise-demo');
    expect(packageJson.scripts?.doctor).toBe('volt doctor');
    expect(packageJson.scripts?.package).toBe('volt package');
    expect(config).toContain('name: "Enterprise Demo"');
    expect(config).toContain('title: "Enterprise Demo"');
    expect(config).toContain('package: {');
    expect(config).toContain("installMode: 'perMachine'");
    expect(config).toContain('generateAdmx: true');
  });

  it('exits with code 1 when target directory already exists', async () => {
    const workspaceRoot = createTempDir('create-volt-existing-');
    const previousCwd = process.cwd();
    mkdirSync(resolve(workspaceRoot, 'my-existing-app'), { recursive: true });

    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      const normalized = code === undefined || code === null ? 0 : Number(code);
      throw new Error(`__PROCESS_EXIT__${normalized}`);
    }) as never);

    process.chdir(workspaceRoot);
    try {
      await expect(
        __testOnly.createProject({
          name: 'my-existing-app',
          displayName: 'My Existing App',
          framework: 'vanilla',
        }),
      ).rejects.toThrow('__PROCESS_EXIT__1');
    } finally {
      process.chdir(previousCwd);
      exitSpy.mockRestore();
    }
  });

  it('exits with code 1 when framework template is missing', async () => {
    const workspaceRoot = createTempDir('create-volt-missing-template-');
    const previousCwd = process.cwd();

    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      const normalized = code === undefined || code === null ? 0 : Number(code);
      throw new Error(`__PROCESS_EXIT__${normalized}`);
    }) as never);

    process.chdir(workspaceRoot);
    try {
      await expect(
        __testOnly.createProject({
          name: 'bad-template-app',
          displayName: 'Bad Template App',
          framework: 'missing-template' as never,
        }),
      ).rejects.toThrow('__PROCESS_EXIT__1');
    } finally {
      process.chdir(previousCwd);
      exitSpy.mockRestore();
    }
  });
});
