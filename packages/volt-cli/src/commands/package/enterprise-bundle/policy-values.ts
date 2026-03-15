import { ENTERPRISE_POLICY_SCHEMA } from '../enterprise-schema.js';
import type { EnterpriseBundleOptions } from './types.js';

export function resolvePolicyValues(options: EnterpriseBundleOptions): Record<string, unknown> {
  const values: Record<string, unknown> = {};
  for (const policy of ENTERPRISE_POLICY_SCHEMA.policies) {
    const fromConfig =
      policy.id === 'InstallMode'
        ? options.installMode
        : readValueAtPath(options.config as unknown as Record<string, unknown>, policy.configPath);
    values[policy.id] = fromConfig ?? policy.defaultValue ?? null;
  }
  return values;
}

function readValueAtPath(value: Record<string, unknown>, path: string): unknown {
  const segments = path.split('.');
  let current: unknown = value;
  for (const segment of segments) {
    if (!current || typeof current !== 'object') {
      return undefined;
    }
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}
