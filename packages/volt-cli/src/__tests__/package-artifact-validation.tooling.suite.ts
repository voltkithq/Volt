import { describe, expect, it, vi } from 'vitest';
import { __testOnly } from '../commands/package.js';

describe('packaging tool execution helpers', () => {
  it('identifies command-not-found failures by ENOENT code', () => {
    expect(__testOnly.isMissingExecutableError({ code: 'ENOENT' })).toBe(true);
    expect(__testOnly.isMissingExecutableError({ code: 'EACCES' })).toBe(false);
    expect(__testOnly.isMissingExecutableError(new Error('boom'))).toBe(false);
  });

  it('treats ENOENT packager failures as missing-tool warnings', () => {
    let missingToolHandlerCalled = false;
    let exitCalled = false;
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      exitCalled = true;
      const normalized = code === undefined || code === null ? 0 : Number(code);
      throw new Error(`__PROCESS_EXIT__${normalized}`);
    }) as never);

    expect(() => {
      __testOnly.runPackagingTool(
        'makensis',
        ['installer.nsi'],
        () => {
          missingToolHandlerCalled = true;
        },
        '[volt] Failed to create Windows NSIS installer.',
        () => {
          const error = new Error('missing command') as NodeJS.ErrnoException;
          error.code = 'ENOENT';
          throw error;
        },
      );
    }).not.toThrow();

    expect(missingToolHandlerCalled).toBe(true);
    expect(exitCalled).toBe(false);
    exitSpy.mockRestore();
  });

  it('fails fast when a packager exists but exits with an error', () => {
    const missingToolSpy = vi.fn();
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      const normalized = code === undefined || code === null ? 0 : Number(code);
      throw new Error(`__PROCESS_EXIT__${normalized}`);
    }) as never);

    expect(() => {
      __testOnly.runPackagingTool(
        'makensis',
        ['installer.nsi'],
        missingToolSpy,
        '[volt] Failed to create Windows NSIS installer.',
        () => {
          const error = new Error('command failed') as NodeJS.ErrnoException;
          error.code = 'EACCES';
          throw error;
        },
      );
    }).toThrow('__PROCESS_EXIT__1');

    expect(missingToolSpy).not.toHaveBeenCalled();
    expect(errorSpy).toHaveBeenCalled();
    exitSpy.mockRestore();
    errorSpy.mockRestore();
  });

  it('falls back to secondary packaging tool when primary command is missing', () => {
    const execute = vi.fn()
      .mockImplementationOnce(() => {
        const error = new Error('missing primary') as NodeJS.ErrnoException;
        error.code = 'ENOENT';
        throw error;
      })
      .mockImplementationOnce(() => {});
    const missingBothSpy = vi.fn();

    expect(() => {
      __testOnly.runPackagingToolWithFallback(
        { command: 'makemsix', args: ['pack'] },
        { command: 'makeappx', args: ['pack'] },
        missingBothSpy,
        '[volt] Failed to create Windows MSIX package.',
        execute,
      );
    }).not.toThrow();

    expect(execute).toHaveBeenCalledTimes(2);
    expect(execute).toHaveBeenNthCalledWith(1, 'makemsix', ['pack'], { stdio: 'inherit' });
    expect(execute).toHaveBeenNthCalledWith(2, 'makeappx', ['pack'], { stdio: 'inherit' });
    expect(missingBothSpy).not.toHaveBeenCalled();
  });

  it('invokes missing-tool callback when both primary and fallback commands are unavailable', () => {
    const execute = vi.fn().mockImplementation(() => {
      const error = new Error('missing command') as NodeJS.ErrnoException;
      error.code = 'ENOENT';
      throw error;
    });
    const missingBothSpy = vi.fn();

    expect(() => {
      __testOnly.runPackagingToolWithFallback(
        { command: 'makemsix', args: ['pack'] },
        { command: 'makeappx', args: ['pack'] },
        missingBothSpy,
        '[volt] Failed to create Windows MSIX package.',
        execute,
      );
    }).not.toThrow();

    expect(execute).toHaveBeenCalledTimes(2);
    expect(missingBothSpy).toHaveBeenCalledTimes(1);
  });

  it('fails fast when fallback tool exists but returns an execution error', () => {
    const execute = vi.fn()
      .mockImplementationOnce(() => {
        const error = new Error('missing primary') as NodeJS.ErrnoException;
        error.code = 'ENOENT';
        throw error;
      })
      .mockImplementationOnce(() => {
        const error = new Error('fallback failed') as NodeJS.ErrnoException;
        error.code = 'EACCES';
        throw error;
      });
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
      const normalized = code === undefined || code === null ? 0 : Number(code);
      throw new Error(`__PROCESS_EXIT__${normalized}`);
    }) as never);

    expect(() => {
      __testOnly.runPackagingToolWithFallback(
        { command: 'makemsix', args: ['pack'] },
        { command: 'makeappx', args: ['pack'] },
        vi.fn(),
        '[volt] Failed to create Windows MSIX package.',
        execute,
      );
    }).toThrow('__PROCESS_EXIT__1');

    expect(errorSpy).toHaveBeenCalled();
    exitSpy.mockRestore();
    errorSpy.mockRestore();
  });
});
