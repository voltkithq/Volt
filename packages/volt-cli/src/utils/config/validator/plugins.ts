import { pushError, type ValidationContext } from './context.js';
import {
  isPlainObject,
  sanitizePermissionArray,
  sanitizeStringArray,
  validatePositiveIntegerField,
} from './shared.js';

const VALID_PLUGIN_SPAWN_STRATEGIES = ['lazy', 'eager'] as const;

export function validatePluginsConfig(
  configRecord: Record<string, unknown>,
  context: ValidationContext,
): void {
  const pluginsRecord = configRecord.plugins;
  if (pluginsRecord === undefined) {
    return;
  }

  if (!isPlainObject(pluginsRecord)) {
    pushError(context, `'plugins' must be an object when provided.`);
    delete configRecord.plugins;
    return;
  }

  const enabled = pluginsRecord.enabled;
  if (enabled !== undefined) {
    if (!Array.isArray(enabled)) {
      pushError(context, `'plugins.enabled' must be an array of non-empty plugin IDs.`);
      delete pluginsRecord.enabled;
    } else {
      pluginsRecord.enabled = sanitizeStringArray(enabled, 'plugins.enabled', context);
    }
  }

  const grants = pluginsRecord.grants;
  if (grants !== undefined) {
    if (!isPlainObject(grants)) {
      pushError(context, `'plugins.grants' must be an object when provided.`);
      delete pluginsRecord.grants;
    } else {
      for (const [pluginId, grantedPermissions] of Object.entries(grants)) {
        if (pluginId.trim().length === 0) {
          pushError(context, `'plugins.grants' keys must be non-empty plugin IDs.`);
          delete grants[pluginId];
          continue;
        }

        if (!Array.isArray(grantedPermissions)) {
          pushError(context, `'plugins.grants.${pluginId}' must be an array of permissions.`);
          delete grants[pluginId];
          continue;
        }

        grants[pluginId] = sanitizePermissionArray(
          grantedPermissions,
          `plugins.grants.${pluginId}`,
          context,
        );
      }
    }
  }

  const pluginDirs = pluginsRecord.pluginDirs;
  if (pluginDirs !== undefined) {
    if (!Array.isArray(pluginDirs)) {
      pushError(context, `'plugins.pluginDirs' must be an array of non-empty paths.`);
      delete pluginsRecord.pluginDirs;
    } else {
      pluginsRecord.pluginDirs = sanitizeStringArray(pluginDirs, 'plugins.pluginDirs', context);
    }
  }

  const limits = pluginsRecord.limits;
  if (limits !== undefined) {
    if (!isPlainObject(limits)) {
      pushError(context, `'plugins.limits' must be an object when provided.`);
      delete pluginsRecord.limits;
    } else {
      validatePositiveIntegerField(
        limits,
        'activationTimeoutMs',
        'plugins.limits.activationTimeoutMs',
        context,
      );
      validatePositiveIntegerField(
        limits,
        'deactivationTimeoutMs',
        'plugins.limits.deactivationTimeoutMs',
        context,
      );
      validatePositiveIntegerField(limits, 'callTimeoutMs', 'plugins.limits.callTimeoutMs', context);
      validatePositiveIntegerField(limits, 'maxPlugins', 'plugins.limits.maxPlugins', context);
      validatePositiveIntegerField(
        limits,
        'heartbeatIntervalMs',
        'plugins.limits.heartbeatIntervalMs',
        context,
      );
      validatePositiveIntegerField(
        limits,
        'heartbeatTimeoutMs',
        'plugins.limits.heartbeatTimeoutMs',
        context,
      );
    }
  }

  const spawning = pluginsRecord.spawning;
  if (spawning !== undefined) {
    if (!isPlainObject(spawning)) {
      pushError(context, `'plugins.spawning' must be an object when provided.`);
      delete pluginsRecord.spawning;
    } else {
      const strategy = spawning.strategy;
      if (strategy !== undefined) {
        if (
          typeof strategy !== 'string'
          || !(VALID_PLUGIN_SPAWN_STRATEGIES as readonly string[]).includes(strategy)
        ) {
          pushError(context, `'plugins.spawning.strategy' must be "lazy" or "eager".`);
          delete spawning.strategy;
        }
      }

      validatePositiveIntegerField(
        spawning,
        'idleTimeoutMs',
        'plugins.spawning.idleTimeoutMs',
        context,
      );

      const preSpawn = spawning.preSpawn;
      if (preSpawn !== undefined) {
        if (!Array.isArray(preSpawn)) {
          pushError(
            context,
            `'plugins.spawning.preSpawn' must be an array of non-empty plugin IDs.`,
          );
          delete spawning.preSpawn;
        } else {
          spawning.preSpawn = sanitizeStringArray(preSpawn, 'plugins.spawning.preSpawn', context);
        }
      }
    }
  }
}
