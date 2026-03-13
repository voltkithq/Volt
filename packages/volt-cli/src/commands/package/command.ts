import { existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { loadConfig } from '../../utils/config.js';
import { toSafeArtifactVersion, toSafeBinaryName } from '../../utils/naming.js';
import {
  resolveSigningConfig,
  type SigningArtifactResult,
} from '../../utils/signing.js';
import {
  normalizePackagePlatform,
  resolveRuntimeArtifactForPackaging,
  validateRuntimeArtifactCompatibility,
} from '../../utils/runtime-artifact.js';
import {
  normalizeWindowsInstallMode,
  parsePackageConfig,
  validateRequestedPackageFormat,
} from './config.js';
import { writeEnterpriseDeploymentBundle } from './enterprise-bundle.js';
import { packageLinux, packageMacOS, packageWindows } from './platform-packagers.js';
import {
  ALLOWED_PACKAGE_FORMATS,
  type PackageArtifactSummary,
  type PackageCommandSummary,
  type PackageEnterpriseConfig,
  type PackageOptions,
  type WindowsInstallMode,
  WINDOWS_UPDATER_HELPER_FILE_NAME,
} from './types.js';
import { runPackagePreflight, enforcePreflightResult } from '../../utils/preflight.js';

/**
 * Package the application into platform-specific installers.
 * Supports: Windows (NSIS), macOS (.app + .dmg), Linux (AppImage, .deb)
 */
export async function packageCommand(options: PackageOptions): Promise<void> {
  const startedAt = new Date().toISOString();
  const startedAtMs = Date.now();
  const cwd = process.cwd();
  console.log('[volt] Packaging application...');

  const config = await loadConfig(cwd, { strict: true, commandName: 'package' });
  const appName = config.name;
  const version = config.version ?? '0.1.0';
  const binaryName = toSafeBinaryName(appName);
  const artifactVersion = toSafeArtifactVersion(version);
  const packageConfig = parsePackageConfig(
    (config as unknown as Record<string, unknown>)['package'],
    appName,
  );

  const distVoltDir = resolve(cwd, 'dist-volt');
  const packageDir = resolve(cwd, 'dist-package');

  const platform = normalizePackagePlatform(options.target);
  const format = validateRequestedPackageFormat(platform, options.format);
  if (options.format && !format) {
    const supported = ALLOWED_PACKAGE_FORMATS[platform].join(', ');
    console.error(
      `[volt] Unsupported package format "${options.format}" for platform "${platform}". Supported formats: ${supported}.`,
    );
    process.exit(1);
  }

  enforcePreflightResult(
    runPackagePreflight(cwd, platform, { format: format ?? undefined, distVoltDir }),
  );

  if (!existsSync(packageDir)) {
    mkdirSync(packageDir, { recursive: true });
  }

  const cliInstallMode = normalizeWindowsInstallMode(options.installMode);
  if (options.installMode && !cliInstallMode) {
    console.error(
      `[volt] Unsupported install mode "${options.installMode}". Supported values: perMachine, perUser.`,
    );
    process.exit(1);
  }

  const resolvedInstallMode: WindowsInstallMode | null = platform === 'win32'
    ? cliInstallMode ?? packageConfig.windows?.installMode ?? 'perMachine'
    : null;
  if (platform !== 'win32' && options.installMode) {
    console.warn('[volt] Ignoring --install-mode for non-Windows package targets.');
  }

  const artifactResolution = resolveRuntimeArtifactForPackaging(distVoltDir, binaryName);
  const runtimeArtifact = artifactResolution.artifact;
  if (!runtimeArtifact) {
    console.error(`[volt] No runtime artifact found in ${distVoltDir}. Run \`volt build\` first.`);
    if (artifactResolution.attemptedPaths.length > 0) {
      console.error(`[volt] Checked paths:\n  - ${artifactResolution.attemptedPaths.join('\n  - ')}`);
    }
    process.exit(1);
  }
  const compatibility = validateRuntimeArtifactCompatibility(runtimeArtifact, platform);
  if (!compatibility.ok) {
    console.error(`[volt] ${compatibility.reason}`);
    console.error(
      '[volt] Packaging currently requires a runnable app executable. '
        + 'Current build output is not executable for this target.',
    );
    process.exit(1);
  }

  console.log(`[volt] Platform: ${platform}`);
  console.log(`[volt] App: ${appName} v${version}`);
  console.log(`[volt] Identifier: ${packageConfig.identifier}`);
  console.log(`[volt] Runtime artifact: ${runtimeArtifact.fileName}`);
  if (resolvedInstallMode) {
    console.log(`[volt] Windows install mode: ${resolvedInstallMode}`);
  }

  const signingConfig = resolveSigningConfig(packageConfig, platform);
  const signingResults: SigningArtifactResult[] = [];
  if (signingConfig?.macOS || signingConfig?.windows) {
    console.log('[volt] Code signing: enabled');
  }

  let missingTools: string[];

  if (platform === 'win32') {
    const updaterHelperPath = resolve(distVoltDir, WINDOWS_UPDATER_HELPER_FILE_NAME);
    const updaterHelperFileName = existsSync(updaterHelperPath)
      ? WINDOWS_UPDATER_HELPER_FILE_NAME
      : null;
    missingTools = await packageWindows(
      appName,
      version,
      artifactVersion,
      binaryName,
      distVoltDir,
      packageDir,
      packageConfig,
      runtimeArtifact,
      format,
      packageConfig.windows,
      resolvedInstallMode ?? 'perMachine',
      signingConfig?.windows,
      updaterHelperFileName,
      signingResults,
    );
  } else if (platform === 'darwin') {
    missingTools = await packageMacOS(
      appName,
      version,
      artifactVersion,
      binaryName,
      packageConfig,
      packageDir,
      runtimeArtifact,
      format,
      signingConfig?.macOS,
      signingResults,
    );
  } else {
    missingTools = await packageLinux(
      appName,
      version,
      artifactVersion,
      binaryName,
      packageConfig,
      packageDir,
      runtimeArtifact,
      format,
      options.target,
    );
  }

  const enterpriseBundleDecision = resolveEnterpriseBundleDecision(packageConfig.enterprise);
  let artifacts = collectArtifacts(packageDir);
  if (platform === 'win32' && enterpriseBundleDecision.enabled) {
    if (enterpriseBundleDecision.defaulted) {
      console.log(
        '[volt] Enterprise bundle is enabled by default for Windows packaging. '
          + 'Set package.enterprise.generateAdmx=false and package.enterprise.includeDocsBundle=false to disable.',
      );
    }
    const enterpriseBundle = writeEnterpriseDeploymentBundle({
      appName,
      version,
      packageDir,
      packageConfig,
      config,
      installMode: resolvedInstallMode,
      artifacts,
    });
    if (enterpriseBundle.generatedFiles.length > 0) {
      console.log(`[volt] Enterprise bundle generated: ${enterpriseBundle.bundleDir}`);
    }
    artifacts = collectArtifacts(packageDir);
  }

  const summary: PackageCommandSummary = {
    appName,
    version,
    platform,
    format: format ?? null,
    installMode: resolvedInstallMode,
    identifier: packageConfig.identifier,
    runtimeArtifact: runtimeArtifact.fileName,
    outputDir: packageDir,
    startedAt,
    finishedAt: new Date().toISOString(),
    durationMs: Math.max(0, Date.now() - startedAtMs),
    codeSigningEnabled: Boolean(signingConfig?.macOS || signingConfig?.windows),
    signingResults,
    artifacts,
  };

  if (options.jsonOutput) {
    const jsonOutputPath = resolve(cwd, options.jsonOutput);
    mkdirSync(dirname(jsonOutputPath), { recursive: true });
    writeFileSync(jsonOutputPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
    console.log(`[volt] Package summary JSON written to ${jsonOutputPath}`);
  }

  if (options.json) {
    console.log(JSON.stringify(summary, null, 2));
  }

  if (missingTools.length > 0) {
    // Default format counts match the packager defaults (not ALLOWED_PACKAGE_FORMATS):
    // win32 → ['nsis'], darwin → ['app'], linux → ['appimage', 'deb']
    const formatsAttempted = format ? 1 : platform === 'linux' ? 2 : 1;
    if (missingTools.length >= formatsAttempted) {
      console.log('[volt] Packaging skipped: required tools not found. The built binary is available in dist-volt/.');
    } else {
      console.log(
        `[volt] Packaging partially complete. Some tools were not found: ${missingTools.join(', ')}. Output: ${packageDir}/`,
      );
    }
  } else {
    console.log(`[volt] Packaging complete. Output: ${packageDir}/`);
  }
}

function resolveEnterpriseBundleDecision(config: PackageEnterpriseConfig | undefined): {
  enabled: boolean;
  defaulted: boolean;
} {
  if (!config) {
    return { enabled: true, defaulted: true };
  }
  return {
    enabled: config.generateAdmx !== false || config.includeDocsBundle !== false,
    defaulted: false,
  };
}

function collectArtifacts(packageDir: string): PackageArtifactSummary[] {
  const artifacts: PackageArtifactSummary[] = [];

  const visit = (directory: string): void => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolutePath = resolve(directory, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
        continue;
      }

      const stats = statSync(absolutePath);
      if (!stats.isFile()) {
        continue;
      }

      artifacts.push({
        path: absolutePath,
        fileName: entry.name,
      });
    }
  };

  if (existsSync(packageDir)) {
    visit(packageDir);
  }

  return artifacts;
}
