import type { VoltConfig } from 'voltkit';
import { DEFAULT_CONFIG, VALID_PERMISSIONS } from '../constants.js';
import {
  pushError,
  type ValidationContext,
} from './context.js';

export function validateCoreConfig(
  config: VoltConfig,
  configRecord: Record<string, unknown>,
  context: ValidationContext,
): void {
  if (!config.name || typeof config.name !== 'string') {
    const message = `'name' must be a non-empty string.`;
    pushError(context, message);
    console.error(`[volt] Example: defineConfig({ name: 'My App', ... })`);
    config.name = DEFAULT_CONFIG.name;
  }

  if (config.version !== undefined && typeof config.version !== 'string') {
    pushError(context, `'version' must be a string (e.g., "1.0.0").`);
    config.version = undefined;
  }

  if (configRecord.backend !== undefined) {
    if (typeof configRecord.backend !== 'string' || configRecord.backend.trim().length === 0) {
      pushError(
        context,
        `'backend' must be a non-empty string path (e.g., "./src/backend.ts").`,
      );
      delete configRecord.backend;
    } else {
      configRecord.backend = configRecord.backend.trim();
    }
  }

  if (config.window) {
    const w = config.window;
    if (w.width !== undefined && (typeof w.width !== 'number' || w.width <= 0)) {
      pushError(context, `'window.width' must be a positive number.`);
      w.width = DEFAULT_CONFIG.window?.width;
    }
    if (w.height !== undefined && (typeof w.height !== 'number' || w.height <= 0)) {
      pushError(context, `'window.height' must be a positive number.`);
      w.height = DEFAULT_CONFIG.window?.height;
    }
  }

  if (config.permissions) {
    if (!Array.isArray(config.permissions)) {
      pushError(context, `'permissions' must be an array.`);
      console.error(`[volt] Valid permissions: ${VALID_PERMISSIONS.join(', ')}`);
      config.permissions = [];
    } else {
      const filtered = config.permissions.filter((perm): perm is (typeof VALID_PERMISSIONS)[number] => {
        const valid = VALID_PERMISSIONS.includes(perm);
        if (!valid) {
          const message = `Unknown permission '${perm}'.`;
          pushError(context, message);
          console.error(`[volt] Valid permissions: ${VALID_PERMISSIONS.join(', ')}`);
        }
        return valid;
      });
      config.permissions = filtered;
    }
  }

  const runtime = configRecord.runtime as Record<string, unknown> | undefined;
  if (runtime !== undefined) {
    const poolSize = runtime.poolSize;
    if (
      poolSize !== undefined
      && (typeof poolSize !== 'number' || !Number.isInteger(poolSize) || poolSize <= 0)
    ) {
      pushError(context, `'runtime.poolSize' must be a positive integer.`);
      delete runtime.poolSize;
    }
  }

  const runtimePoolSizeLegacy = configRecord.runtimePoolSize;
  if (
    runtimePoolSizeLegacy !== undefined
    && (typeof runtimePoolSizeLegacy !== 'number'
      || !Number.isInteger(runtimePoolSizeLegacy)
      || runtimePoolSizeLegacy <= 0)
  ) {
    pushError(context, `'runtimePoolSize' must be a positive integer.`);
    delete configRecord.runtimePoolSize;
  }
}
