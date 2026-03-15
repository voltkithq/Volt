import type { DoctorCheckResult, DoctorReport } from './types.js';

export function summarizeDoctorChecks(
  checks: readonly DoctorCheckResult[],
): DoctorReport['summary'] {
  return {
    pass: checks.filter((check) => check.status === 'pass').length,
    warn: checks.filter((check) => check.status === 'warn').length,
    fail: checks.filter((check) => check.status === 'fail').length,
  };
}

export function printDoctorReport(report: DoctorReport): void {
  console.log(`[volt:doctor] Target platform: ${report.target}`);
  console.log(`[volt:doctor] Package formats: ${report.formats.join(', ')}`);
  for (const check of report.checks) {
    console.log(`[volt:doctor] [${check.status.toUpperCase()}] ${check.title}: ${check.details}`);
  }
  console.log(
    `[volt:doctor] Summary: pass=${report.summary.pass} warn=${report.summary.warn} fail=${report.summary.fail}`,
  );
}
