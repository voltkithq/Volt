import { describe, expect, it } from 'vitest';
import { doctorTestOnly } from '../commands/doctor.js';

describe('doctor command checks', () => {
  it('resolves default formats by platform', () => {
    expect(doctorTestOnly.resolveDoctorFormats('win32', undefined)).toEqual(['nsis']);
    expect(doctorTestOnly.resolveDoctorFormats('darwin', undefined)).toEqual(['app']);
    expect(doctorTestOnly.resolveDoctorFormats('linux', undefined)).toEqual(['appimage', 'deb']);
    expect(doctorTestOnly.resolveDoctorFormats('win32', 'msix')).toEqual(['msix']);
  });

  it('fails packaging checks when required windows tools are missing', () => {
    const checks = doctorTestOnly.collectDoctorChecks(
      {
        platform: 'win32',
        formats: ['nsis'],
        packageConfig: {
          identifier: 'com.example.app',
        },
      },
      {
        isToolAvailable: () => false,
        env: {},
      },
    );

    const nsis = checks.find((check) => check.id === 'pkg.win.nsis');
    expect(nsis?.status).toBe('fail');
  });

  it('accepts MSIX check when fallback makeappx tool is available', () => {
    const checks = doctorTestOnly.collectDoctorChecks(
      {
        platform: 'win32',
        formats: ['msix'],
        packageConfig: {
          identifier: 'com.example.app',
        },
      },
      {
        isToolAvailable: (toolName) => toolName === 'cargo' || toolName === 'rustc' || toolName === 'makeappx',
        env: {},
      },
    );

    const msix = checks.find((check) => check.id === 'pkg.win.msix');
    expect(msix?.status).toBe('pass');
  });

  it('reports local windows signing prerequisites as passing when tool and certificate are present', () => {
    const checks = doctorTestOnly.collectDoctorChecks(
      {
        platform: 'win32',
        formats: ['nsis'],
        packageConfig: {
          identifier: 'com.example.app',
          signing: {
            windows: {
              provider: 'local',
              certificate: 'C:/certs/code-signing.pfx',
            },
          },
        },
      },
      {
        isToolAvailable: (toolName) => toolName === 'cargo' || toolName === 'rustc' || toolName === 'signtool' || toolName === 'makensis',
        env: {},
      },
    );

    const toolCheck = checks.find((check) => check.id === 'signing.win.local.tool');
    const certCheck = checks.find((check) => check.id === 'signing.win.local.certificate');
    expect(toolCheck?.status).toBe('pass');
    expect(certCheck?.status).toBe('pass');
  });

  it('reports Azure signing metadata gaps as failures', () => {
    const checks = doctorTestOnly.collectDoctorChecks(
      {
        platform: 'win32',
        formats: ['nsis'],
        packageConfig: {
          identifier: 'com.example.app',
          signing: {
            windows: {
              provider: 'azureTrustedSigning',
            },
          },
        },
      },
      {
        isToolAvailable: (toolName) => toolName === 'cargo' || toolName === 'rustc' || toolName === 'signtool' || toolName === 'makensis',
        env: {},
      },
    );

    const dlib = checks.find((check) => check.id === 'signing.win.azure.dlib');
    const metadata = checks.find((check) => check.id === 'signing.win.azure.metadata');
    expect(dlib?.status).toBe('fail');
    expect(metadata?.status).toBe('fail');
  });

  it('summarizes check status counts', () => {
    const summary = doctorTestOnly.summarizeDoctorChecks([
      { id: 'a', title: 'A', details: 'A', status: 'pass' },
      { id: 'b', title: 'B', details: 'B', status: 'warn' },
      { id: 'c', title: 'C', details: 'C', status: 'fail' },
      { id: 'd', title: 'D', details: 'D', status: 'pass' },
    ]);

    expect(summary).toEqual({
      pass: 2,
      warn: 1,
      fail: 1,
    });
  });
});
