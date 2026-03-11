import { execFileSync } from 'node:child_process';
import type { ExecFileSyncFn } from './types.js';

export function escapeNsisString(value: string): string {
  return value
    .replace(/\$/g, '$$$$')
    .replace(/"/g, '$\\"')
    .replace(/\r?\n|\r/g, ' ');
}

export function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

export function inferDebArchitecture(
  packageTarget: string | undefined,
  rustTarget: string | null,
  hostArch: NodeJS.Architecture = process.arch,
): string {
  const triple = (rustTarget ?? packageTarget ?? '').toLowerCase();
  if (triple.includes('x86_64') || triple.includes('amd64')) {
    return 'amd64';
  }
  if (triple.includes('aarch64') || triple.includes('arm64')) {
    return 'arm64';
  }
  if (triple.includes('armv7') || triple.includes('armhf') || triple.includes('gnueabihf') || triple.includes('eabihf')) {
    return 'armhf';
  }
  if (
    triple.includes('armv6')
    || triple.includes('armel')
    || ((triple.includes('arm-') || triple.startsWith('arm')) && (triple.includes('gnueabi') || triple.includes('eabi')))
  ) {
    return 'armel';
  }
  if (triple.includes('i686') || triple.includes('i386') || triple === 'x86') {
    return 'i386';
  }
  if (triple.includes('riscv64')) {
    return 'riscv64';
  }
  if (triple.includes('ppc64le') || triple.includes('powerpc64le')) {
    return 'ppc64el';
  }
  if (triple.includes('s390x')) {
    return 's390x';
  }

  switch (hostArch) {
    case 'x64':
      return 'amd64';
    case 'arm64':
      return 'arm64';
    case 'ia32':
      return 'i386';
    case 'arm':
      return 'armhf';
    case 'riscv64':
      return 'riscv64';
    default:
      return 'amd64';
  }
}

export function inferAppImageArchitecture(
  packageTarget: string | undefined,
  rustTarget: string | null,
  hostArch: NodeJS.Architecture = process.arch,
): string {
  const triple = (rustTarget ?? packageTarget ?? '').toLowerCase();
  if (triple.includes('x86_64') || triple.includes('amd64')) {
    return 'x86_64';
  }
  if (triple.includes('aarch64') || triple.includes('arm64')) {
    return 'aarch64';
  }
  if (
    triple.includes('armv7')
    || triple.includes('armhf')
    || triple.includes('gnueabihf')
    || triple.includes('eabihf')
    || triple.includes('armv6')
    || triple.includes('armel')
    || ((triple.includes('arm-') || triple.startsWith('arm')) && (triple.includes('gnueabi') || triple.includes('eabi')))
  ) {
    return 'armhf';
  }
  if (triple.includes('i686') || triple.includes('i386') || triple === 'x86') {
    return 'i686';
  }
  if (triple.includes('riscv64')) {
    return 'riscv64';
  }
  if (triple.includes('ppc64le') || triple.includes('powerpc64le')) {
    return 'ppc64le';
  }
  if (triple.includes('s390x')) {
    return 's390x';
  }

  switch (hostArch) {
    case 'x64':
      return 'x86_64';
    case 'arm64':
      return 'aarch64';
    case 'ia32':
      return 'i686';
    case 'arm':
      return 'armhf';
    case 'riscv64':
      return 'riscv64';
    default:
      return 'x86_64';
  }
}

export function normalizeDebianControlVersion(version: string): string {
  const normalized = version
    .trim()
    .replace(/[^0-9A-Za-z.+:~-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^[^0-9A-Za-z]+|[^0-9A-Za-z]+$/g, '');

  return normalized || '0.1.0';
}

export function isMissingExecutableError(error: unknown): boolean {
  if (!error || typeof error !== 'object') {
    return false;
  }
  return (error as { code?: unknown }).code === 'ENOENT';
}

export function runPackagingTool(
  command: string,
  args: readonly string[],
  onMissingTool: () => void,
  failureMessage: string,
  execute: ExecFileSyncFn = execFileSync as ExecFileSyncFn,
): boolean {
  try {
    execute(command, args, { stdio: 'inherit' });
    return true;
  } catch (error) {
    if (isMissingExecutableError(error)) {
      onMissingTool();
      return false;
    }
    console.error(failureMessage, error);
    process.exit(1);
  }
}

export function runPackagingToolWithFallback(
  primary: { command: string; args: readonly string[] },
  fallback: { command: string; args: readonly string[] },
  onMissingBoth: () => void,
  failureMessage: string,
  execute: ExecFileSyncFn = execFileSync as ExecFileSyncFn,
): boolean {
  try {
    execute(primary.command, primary.args, { stdio: 'inherit' });
    return true;
  } catch (error) {
    if (!isMissingExecutableError(error)) {
      console.error(failureMessage, error);
      process.exit(1);
    }
  }

  try {
    execute(fallback.command, fallback.args, { stdio: 'inherit' });
    return true;
  } catch (fallbackError) {
    if (isMissingExecutableError(fallbackError)) {
      onMissingBoth();
      return false;
    }
    console.error(failureMessage, fallbackError);
    process.exit(1);
  }
}

export function normalizeMsixVersion(version: string): string {
  const parts = version
    .split('.')
    .map((part) => Number.parseInt(part.replace(/[^0-9].*$/, ''), 10))
    .filter((part) => Number.isFinite(part) && part >= 0)
    .slice(0, 4);

  while (parts.length < 4) {
    parts.push(0);
  }

  return parts.join('.');
}
