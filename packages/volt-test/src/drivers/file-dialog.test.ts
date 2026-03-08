import { describe, expect, it } from 'vitest';
import { FileDialogAutomationDriver } from './file-dialog.js';

describe('FileDialogAutomationDriver', () => {
  it('parses and normalizes open dialog payloads on windows', () => {
    const driver = new FileDialogAutomationDriver({ platform: 'win32' });
    const result = driver.parseOpenDialogResult({
      canceled: false,
      filePaths: ['c:/work/demo/file-a.txt', 'D:\\demo\\file-b.txt'],
    });

    expect(result).toEqual({
      canceled: false,
      filePaths: ['C:\\work\\demo\\file-a.txt', 'D:\\demo\\file-b.txt'],
    });
  });

  it('parses and normalizes save dialog payloads on posix', () => {
    const driver = new FileDialogAutomationDriver({ platform: 'linux' });
    const result = driver.parseSaveDialogResult({
      canceled: false,
      filePath: '\\tmp\\result.json',
    });

    expect(result).toEqual({
      canceled: false,
      filePath: '/tmp/result.json',
    });
  });

  it('rejects inconsistent canceled payloads', () => {
    const driver = new FileDialogAutomationDriver({ platform: 'darwin' });
    expect(() =>
      driver.parseOpenDialogResult({
        canceled: true,
        filePaths: ['/tmp/a.txt'],
      }),
    ).toThrow('canceled dialog');

    expect(() =>
      driver.parseSaveDialogResult({
        canceled: true,
        filePath: '/tmp/a.txt',
      }),
    ).toThrow('canceled dialog');
  });

  it('asserts expected selected paths', () => {
    const driver = new FileDialogAutomationDriver({ platform: 'linux' });
    const openResult = driver.parseOpenDialogResult({
      canceled: false,
      filePaths: ['/tmp/a.txt', '/tmp/b.txt'],
    });
    driver.assertOpenSelection(openResult, ['/tmp/a.txt', '/tmp/b.txt']);

    const saveResult = driver.parseSaveDialogResult({
      canceled: false,
      filePath: '/tmp/result.txt',
    });
    driver.assertSaveSelection(saveResult, '/tmp/result.txt');
  });
});
