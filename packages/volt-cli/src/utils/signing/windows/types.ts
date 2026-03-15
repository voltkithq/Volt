import type { ResolvedWindowsConfig, SigningArtifactResult } from '../types.js';

export type VerificationTool = 'osslsigncode' | 'signtool';

export type SigningResultCore = Omit<SigningArtifactResult, 'startedAt' | 'finishedAt'>;

export interface WindowsSigningContext {
  exePath: string;
  config: ResolvedWindowsConfig;
}
