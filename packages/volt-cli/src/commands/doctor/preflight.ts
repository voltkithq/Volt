import type { PreflightResult } from '../../utils/preflight.js';

import type { DoctorCheckResult } from './types.js';

export function mergePreflightChecks(
  checks: DoctorCheckResult[],
  buildPreflight: PreflightResult,
  packagePreflight: PreflightResult,
): void {
  for (const error of [...buildPreflight.errors, ...packagePreflight.errors]) {
    if (checks.some((check) => check.id === error.id)) continue;
    checks.push({
      id: error.id,
      status: 'fail',
      title: error.message,
      details: error.fix ?? '',
    });
  }

  for (const warning of [...buildPreflight.warnings, ...packagePreflight.warnings]) {
    if (checks.some((check) => check.id === warning.id)) continue;
    checks.push({
      id: warning.id,
      status: 'warn',
      title: warning.message,
      details: '',
    });
  }
}
