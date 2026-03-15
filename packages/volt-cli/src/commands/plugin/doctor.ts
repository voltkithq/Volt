import { existsSync, readFileSync } from 'node:fs';
import { relative } from 'node:path';
import type { Permission, VoltConfig } from 'voltkit';
import { loadConfig } from '../../utils/config.js';
import { type PluginManifest, validatePluginManifest } from '../../utils/plugin-manifest.js';
import { findNearestVoltAppRoot, resolvePluginBundlePath, resolvePluginManifestPath, resolvePluginProjectRoot, resolvePluginSourceEntry, resolveVoltCliVersion } from './project.js';
import { semverSatisfies } from './semver.js';

export type PluginDoctorStatus = 'pass' | 'warn' | 'fail';

export interface PluginDoctorCheck {
  id: string;
  status: PluginDoctorStatus;
  title: string;
  detail: string;
}

const SUPPORTED_PLUGIN_API_VERSIONS = new Set([1]);

export async function pluginDoctorCommand(): Promise<void> {
  const checks = await collectPluginDoctorChecks(process.cwd());
  for (const check of checks) {
    const icon = check.status === 'pass' ? 'PASS' : check.status === 'warn' ? 'WARN' : 'FAIL';
    console.log(`[volt:plugin:doctor] ${icon} ${check.title}: ${check.detail}`);
  }
  if (checks.some((check) => check.status === 'fail')) {
    process.exit(1);
  }
}

export async function collectPluginDoctorChecks(cwd: string): Promise<PluginDoctorCheck[]> {
  const projectRoot = resolvePluginProjectRoot(cwd);
  const manifestResult = loadDoctorManifest(projectRoot);
  if ('checks' in manifestResult) {
    return manifestResult.checks;
  }

  const project = manifestResult.project;
  const sourceEntry = tryResolve(() => resolvePluginSourceEntry(project.projectRoot));
  const bundlePath = resolvePluginBundlePath(project.projectRoot, project.manifest);
  const currentVoltVersion = resolveVoltCliVersion();
  const appRoot = findNearestVoltAppRoot(project.projectRoot);
  const appConfig = appRoot ? await loadConfig(appRoot, { strict: false, commandName: 'plugin doctor' }) : null;

  return [
    {
      id: 'manifest.schema',
      status: 'pass',
      title: 'Manifest schema',
      detail: relative(project.projectRoot, project.manifestPath) || project.manifestPath,
    },
    {
      id: 'backend.source',
      status: sourceEntry ? 'pass' : 'fail',
      title: 'Source entry',
      detail: sourceEntry ?? 'Expected src/plugin.ts, src/plugin.js, plugin.ts, or plugin.js',
    },
    {
      id: 'backend.bundle',
      status: project.manifest.backend.endsWith('.js') || project.manifest.backend.endsWith('.mjs') ? 'pass' : 'fail',
      title: 'Bundle path',
      detail: bundlePath,
    },
    {
      id: 'api.version',
      status: SUPPORTED_PLUGIN_API_VERSIONS.has(project.manifest.apiVersion) ? 'pass' : 'fail',
      title: 'API version',
      detail: SUPPORTED_PLUGIN_API_VERSIONS.has(project.manifest.apiVersion)
        ? `apiVersion ${project.manifest.apiVersion} is supported`
        : `apiVersion ${project.manifest.apiVersion} is not supported by Volt ${currentVoltVersion}`,
    },
    {
      id: 'engine.volt',
      status: semverSatisfies(currentVoltVersion, project.manifest.engine.volt) ? 'pass' : 'fail',
      title: 'Volt version range',
      detail: semverSatisfies(currentVoltVersion, project.manifest.engine.volt)
        ? `Volt ${currentVoltVersion} satisfies ${project.manifest.engine.volt}`
        : `Volt ${currentVoltVersion} does not satisfy ${project.manifest.engine.volt}`,
    },
    ...collectCompatibilityChecks(project.manifest.id, project.manifest.capabilities, appRoot, appConfig),
    {
      id: 'bundle.exists',
      status: existsSync(bundlePath) ? 'pass' : 'warn',
      title: 'Built bundle',
      detail: existsSync(bundlePath)
        ? `Found ${relative(project.projectRoot, bundlePath) || bundlePath}`
        : `Missing ${relative(project.projectRoot, bundlePath) || bundlePath}. Run \`volt plugin build\`.`,
    },
  ];
}

function loadDoctorManifest(
  projectRoot: string,
): { project: { projectRoot: string; manifestPath: string; manifest: PluginManifest } } | { checks: PluginDoctorCheck[] } {
  const manifestPath = resolvePluginManifestPath(projectRoot);
  if (!existsSync(manifestPath)) {
    return {
      checks: [
        {
          id: 'manifest.schema',
          status: 'fail',
          title: 'Manifest schema',
          detail: `Missing volt-plugin.json in ${projectRoot}`,
        },
      ],
    };
  }

  const rawManifest = tryReadManifest(manifestPath);
  if (!rawManifest.ok) {
    return {
      checks: [
        {
          id: 'manifest.schema',
          status: 'fail',
          title: 'Manifest schema',
          detail: rawManifest.message,
        },
      ],
    };
  }

  const validation = validatePluginManifest(rawManifest.value);
  if (!validation.valid || !validation.manifest) {
    return {
      checks: [
        {
          id: 'manifest.schema',
          status: 'fail',
          title: 'Manifest schema',
          detail: validation.errors.map((error) => `${error.field}: ${error.message}`).join('; '),
        },
      ],
    };
  }

  return {
    project: {
      projectRoot,
      manifestPath,
      manifest: validation.manifest,
    },
  };
}

function tryReadManifest(
  manifestPath: string,
): { ok: true; value: unknown } | { ok: false; message: string } {
  try {
    return {
      ok: true,
      value: JSON.parse(readFileSync(manifestPath, 'utf8')) as unknown,
    };
  } catch (error) {
    return {
      ok: false,
      message: error instanceof Error ? error.message : String(error),
    };
  }
}

function collectCompatibilityChecks(
  pluginId: string,
  capabilities: Permission[],
  appRoot: string | null,
  appConfig: VoltConfig | null,
): PluginDoctorCheck[] {
  if (!appRoot || !appConfig) {
    return [
      {
        id: 'host.config',
        status: 'warn',
        title: 'Host app compatibility',
        detail: 'No parent Volt app config found. Capability compatibility was not checked.',
      },
    ];
  }

  const appPermissions = new Set((appConfig.permissions ?? []) as Permission[]);
  const granted = new Set((appConfig.plugins?.grants?.[pluginId] ?? []) as Permission[]);
  const missingPermissions = capabilities.filter((capability) => !appPermissions.has(capability));
  const missingGrants = capabilities.filter((capability) => !granted.has(capability));
  const checks: PluginDoctorCheck[] = [
    {
      id: 'host.root',
      status: 'pass',
      title: 'Host app root',
      detail: appRoot,
    },
    {
      id: 'host.permissions',
      status: missingPermissions.length === 0 ? 'pass' : 'fail',
      title: 'App permissions',
      detail:
        missingPermissions.length === 0
          ? 'All requested plugin capabilities are declared by the app'
          : `App is missing permissions: ${missingPermissions.join(', ')}`,
    },
  ];

  checks.push({
    id: 'host.grants',
    status: missingGrants.length === 0 ? 'pass' : 'warn',
    title: 'Plugin grants',
    detail:
      missingGrants.length === 0
        ? `App grants all requested capabilities to ${pluginId}`
        : `App does not currently grant: ${missingGrants.join(', ')}`,
  });

  return checks;
}

function tryResolve<T>(factory: () => T): T | null {
  try {
    return factory();
  } catch {
    return null;
  }
}

export const __testOnly = {
  collectPluginDoctorChecks,
};
