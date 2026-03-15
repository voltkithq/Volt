import type { PackageConfig } from '../package/types.js';

export interface DoctorOptions {
  target?: string;
  format?: string;
  json?: boolean;
}

export type DoctorPlatform = 'win32' | 'darwin' | 'linux';
export type DoctorCheckStatus = 'pass' | 'warn' | 'fail';

export interface DoctorCheckResult {
  id: string;
  status: DoctorCheckStatus;
  title: string;
  details: string;
}

export interface DoctorReport {
  target: DoctorPlatform;
  formats: string[];
  checks: DoctorCheckResult[];
  summary: {
    pass: number;
    warn: number;
    fail: number;
  };
}

export interface DoctorDeps {
  isToolAvailable: (toolName: string) => boolean;
  env: NodeJS.ProcessEnv;
}

export interface DoctorCheckContext {
  platform: DoctorPlatform;
  formats: readonly string[];
  packageConfig: PackageConfig;
}
