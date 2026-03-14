import type { LoadConfigOptions } from '../types.js';

export interface ValidationContext {
  errors: string[];
  filename: string;
  strict: boolean;
}

export function createValidationContext(
  filename: string,
  options: LoadConfigOptions,
): ValidationContext {
  return {
    errors: [],
    filename,
    strict: options.strict ?? false,
  };
}

export function pushError(context: ValidationContext, message: string): void {
  console.error(`[volt] Error in ${context.filename}: ${message}`);
  context.errors.push(message);
}

export function pushWarning(context: ValidationContext, message: string): void {
  console.warn(`[volt] Warning in ${context.filename}: ${message}`);
}

export function finalizeValidation(context: ValidationContext): void {
  if (context.strict && context.errors.length > 0) {
    throw new Error(
      `[volt] Invalid configuration in ${context.filename}:\n${context.errors
        .map((error) => `- ${error}`)
        .join('\n')}`,
    );
  }
}
