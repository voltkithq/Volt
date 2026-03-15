import { loadConfig } from '../../utils/config.js';
import { normalizePackagePlatform } from '../../utils/runtime-artifact.js';
import { runBuildPreflight, runPackagePreflight } from '../../utils/preflight.js';
import { parsePackageConfig, validateRequestedPackageFormat } from '../package/config.js';

import { collectDoctorChecks } from './checks.js';
import { resolveDoctorFormats } from './formats.js';
import { mergePreflightChecks } from './preflight.js';
import { printDoctorReport, summarizeDoctorChecks } from './report.js';
import type {
  DoctorCheckResult,
  DoctorCheckStatus,
  DoctorOptions,
  DoctorPlatform,
  DoctorReport,
} from './types.js';

export async function doctorCommand(options: DoctorOptions): Promise<void> {
  const cwd = process.cwd();
  const config = await loadConfig(cwd, { strict: false, commandName: 'doctor' });
  const platform = normalizePackagePlatform(options.target);
  const requestedFormat = validateRequestedPackageFormat(platform, options.format);

  if (options.format && !requestedFormat) {
    const supported = resolveDoctorFormats(platform, undefined).join(', ');
    console.error(
      `[volt:doctor] Unsupported package format "${options.format}" for platform "${platform}". Supported formats: ${supported}.`,
    );
    process.exit(1);
  }

  const packageConfig = parsePackageConfig(
    (config as unknown as Record<string, unknown>)['package'],
    config.name,
  );
  const formats = resolveDoctorFormats(platform, requestedFormat);
  const checks = collectDoctorChecks(
    { platform, formats, packageConfig },
    { isToolAvailable: (await import('../../utils/signing.js')).isToolAvailable, env: process.env },
  );

  mergePreflightChecks(
    checks,
    runBuildPreflight(cwd, config, { target: options.target }),
    runPackagePreflight(cwd, platform, { format: requestedFormat }),
  );

  const report: DoctorReport = {
    target: platform,
    formats,
    checks,
    summary: summarizeDoctorChecks(checks),
  };

  if (options.json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    printDoctorReport(report);
  }

  if (report.summary.fail > 0) {
    process.exit(1);
  }
}

export const __testOnly = {
  resolveDoctorFormats,
  collectDoctorChecks,
  summarizeDoctorChecks,
};

export type { DoctorCheckResult, DoctorCheckStatus, DoctorOptions, DoctorPlatform, DoctorReport };
