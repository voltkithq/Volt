import { VALID_PERMISSIONS } from '../constants.js';
import { pushError, type ValidationContext } from './context.js';

export function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function sanitizeStringArray(
  values: unknown[],
  fieldPath: string,
  context: ValidationContext,
): string[] {
  const normalized: string[] = [];

  for (const value of values) {
    if (typeof value !== 'string' || value.trim().length === 0) {
      pushError(context, `'${fieldPath}' entries must be non-empty strings.`);
      continue;
    }

    const trimmed = value.trim();
    if (!normalized.includes(trimmed)) {
      normalized.push(trimmed);
    }
  }

  return normalized;
}

export function sanitizePermissionArray(
  values: unknown[],
  fieldPath: string,
  context: ValidationContext,
): (typeof VALID_PERMISSIONS)[number][] {
  const normalized: (typeof VALID_PERMISSIONS)[number][] = [];
  const validPermissions = VALID_PERMISSIONS as readonly string[];

  for (const value of values) {
    if (typeof value !== 'string') {
      pushError(context, `'${fieldPath}' entries must be valid permissions.`);
      continue;
    }

    if (!validPermissions.includes(value)) {
      const message = `Unknown permission '${value}' in '${fieldPath}'.`;
      pushError(context, message);
      console.error(`[volt] Valid permissions: ${VALID_PERMISSIONS.join(', ')}`);
      continue;
    }

    const permission = value as (typeof VALID_PERMISSIONS)[number];
    if (!normalized.includes(permission)) {
      normalized.push(permission);
    }
  }

  return normalized;
}

export function validatePositiveIntegerField(
  record: Record<string, unknown>,
  key: string,
  fieldPath: string,
  context: ValidationContext,
): void {
  const value = record[key];
  if (value === undefined) {
    return;
  }

  if (typeof value !== "number" || !Number.isInteger(value) || value <= 0) {
    pushError(context, `'${fieldPath}' must be a positive integer.`);
    delete record[key];
  }
}
