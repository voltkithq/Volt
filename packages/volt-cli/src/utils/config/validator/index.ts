import type { VoltConfig } from 'voltkit';
import type { LoadConfigOptions } from '../types.js';
import { createValidationContext, finalizeValidation } from './context.js';
import { validateCoreConfig } from './core.js';
import { validatePackageConfig } from './package.js';
import { validatePluginsConfig } from './plugins.js';
import { validateSigningConfig } from './signing.js';
import { validateUpdaterConfig } from './updater.js';

export function validateConfig(
  config: VoltConfig,
  filename: string,
  options: LoadConfigOptions,
): VoltConfig {
  const context = createValidationContext(filename, options);
  const configRecord = config as unknown as Record<string, unknown>;

  validateCoreConfig(config, configRecord, context);
  validateUpdaterConfig(config, context);
  validatePluginsConfig(configRecord, context);
  validatePackageConfig(configRecord, context);
  validateSigningConfig(config, context);
  finalizeValidation(context);

  return config;
}
