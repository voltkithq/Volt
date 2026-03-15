/**
 * Plugin manifest (volt-plugin.json) schema validation.
 *
 * Validates the structure defined in the plugin system architecture spec:
 * id, name, version, apiVersion, engine.volt, backend, capabilities, contributes.
 */

const KNOWN_CAPABILITIES = [
  'clipboard',
  'notification',
  'dialog',
  'fs',
  'db',
  'menu',
  'shell',
  'http',
  'globalShortcut',
  'tray',
  'secureStorage',
] as const;

type KnownCapability = (typeof KNOWN_CAPABILITIES)[number];

export interface PluginContributedCommand {
  id: string;
  title: string;
}

export interface PluginContributes {
  commands?: PluginContributedCommand[];
}

export interface PluginSignature {
  algorithm: string;
  value: string;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  apiVersion: number;
  engine: {
    volt: string;
  };
  backend: string;
  capabilities: KnownCapability[];
  prefetchOn?: string[];
  contributes?: PluginContributes;
  signature?: PluginSignature;
}

export interface ManifestValidationError {
  field: string;
  message: string;
}

export interface ManifestValidationResult {
  valid: boolean;
  errors: ManifestValidationError[];
  manifest?: PluginManifest;
}

const REVERSE_DOMAIN_RE = /^[a-z][a-z0-9]*(\.[a-z][a-z0-9]*)+$/;

const SEMVER_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

const SEMVER_RANGE_RE =
  /^(?:[~^]|>=?|<=?|=)?\d+(?:\.\d+(?:\.\d+)?)?(?:-[0-9a-zA-Z.-]+)?(?:\+[0-9a-zA-Z.-]+)?(?:\s+(?:&&|\|\|)\s+(?:[~^]|>=?|<=?|=)?\d+(?:\.\d+(?:\.\d+)?)?(?:-[0-9a-zA-Z.-]+)?(?:\+[0-9a-zA-Z.-]+)?)*$/;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function validatePluginManifest(input: unknown): ManifestValidationResult {
  const errors: ManifestValidationError[] = [];

  if (!isPlainObject(input)) {
    return {
      valid: false,
      errors: [{ field: '(root)', message: 'Manifest must be a JSON object' }],
    };
  }

  // id
  if (typeof input.id !== 'string' || input.id.length === 0) {
    errors.push({ field: 'id', message: 'Must be a non-empty string' });
  } else if (!REVERSE_DOMAIN_RE.test(input.id)) {
    errors.push({
      field: 'id',
      message:
        'Must be in reverse-domain format (e.g. "acme.notes.search"). ' +
        'Segments must start with a lowercase letter and contain only lowercase alphanumerics.',
    });
  }

  // name
  if (typeof input.name !== 'string' || input.name.trim().length === 0) {
    errors.push({ field: 'name', message: 'Must be a non-empty string' });
  }

  // version
  if (typeof input.version !== 'string' || input.version.length === 0) {
    errors.push({ field: 'version', message: 'Must be a non-empty string' });
  } else if (!SEMVER_RE.test(input.version)) {
    errors.push({
      field: 'version',
      message: 'Must be a valid semver string (e.g. "1.0.0")',
    });
  }

  // apiVersion
  if (typeof input.apiVersion !== 'number' || !Number.isInteger(input.apiVersion)) {
    errors.push({ field: 'apiVersion', message: 'Must be an integer' });
  } else if (input.apiVersion < 1) {
    errors.push({ field: 'apiVersion', message: 'Must be >= 1' });
  }

  // engine
  if (!isPlainObject(input.engine)) {
    errors.push({ field: 'engine', message: 'Must be an object with a "volt" field' });
  } else {
    if (typeof input.engine.volt !== 'string' || input.engine.volt.length === 0) {
      errors.push({ field: 'engine.volt', message: 'Must be a non-empty semver range string' });
    } else if (!SEMVER_RANGE_RE.test(input.engine.volt)) {
      errors.push({
        field: 'engine.volt',
        message: 'Must be a valid semver range (e.g. ">=0.2.0")',
      });
    }
  }

  // backend
  if (typeof input.backend !== 'string' || input.backend.length === 0) {
    errors.push({ field: 'backend', message: 'Must be a non-empty file path string' });
  } else if (!input.backend.endsWith('.js') && !input.backend.endsWith('.mjs')) {
    errors.push({
      field: 'backend',
      message: 'Must end with .js or .mjs',
    });
  }

  // capabilities
  if (!Array.isArray(input.capabilities)) {
    errors.push({ field: 'capabilities', message: 'Must be an array' });
  } else {
    for (let i = 0; i < input.capabilities.length; i++) {
      const cap = input.capabilities[i];
      if (typeof cap !== 'string') {
        errors.push({ field: `capabilities[${i}]`, message: 'Must be a string' });
      } else if (!(KNOWN_CAPABILITIES as readonly string[]).includes(cap)) {
        errors.push({
          field: `capabilities[${i}]`,
          message: `Unknown capability "${cap}". Known: ${KNOWN_CAPABILITIES.join(', ')}`,
        });
      }
    }

    if (errors.filter((e) => e.field.startsWith('capabilities')).length === 0) {
      const seen = new Set<string>();
      for (let i = 0; i < input.capabilities.length; i++) {
        const cap = input.capabilities[i] as string;
        if (seen.has(cap)) {
          errors.push({
            field: `capabilities[${i}]`,
            message: `Duplicate capability "${cap}"`,
          });
        }
        seen.add(cap);
      }
    }
  }

  // prefetchOn (optional)
  if (input.prefetchOn !== undefined) {
    if (!Array.isArray(input.prefetchOn)) {
      errors.push({ field: 'prefetchOn', message: 'Must be an array if present' });
    } else {
      for (let i = 0; i < input.prefetchOn.length; i++) {
        const surface = input.prefetchOn[i];
        if (typeof surface !== 'string' || surface.trim().length === 0) {
          errors.push({
            field: `prefetchOn[${i}]`,
            message: 'Must be a non-empty string',
          });
        }
      }
    }
  }

  // contributes (optional)
  if (input.contributes !== undefined) {
    if (!isPlainObject(input.contributes)) {
      errors.push({ field: 'contributes', message: 'Must be an object if present' });
    } else {
      if (input.contributes.commands !== undefined) {
        if (!Array.isArray(input.contributes.commands)) {
          errors.push({ field: 'contributes.commands', message: 'Must be an array if present' });
        } else {
          for (let i = 0; i < input.contributes.commands.length; i++) {
            const cmd = input.contributes.commands[i];
            if (!isPlainObject(cmd)) {
              errors.push({
                field: `contributes.commands[${i}]`,
                message: 'Must be an object with "id" and "title"',
              });
            } else {
              if (typeof cmd.id !== 'string' || cmd.id.trim().length === 0) {
                errors.push({
                  field: `contributes.commands[${i}].id`,
                  message: 'Must be a non-empty string',
                });
              }
              if (typeof cmd.title !== 'string' || cmd.title.trim().length === 0) {
                errors.push({
                  field: `contributes.commands[${i}].title`,
                  message: 'Must be a non-empty string',
                });
              }
            }
          }
        }
      }
    }
  }

  // signature (optional)
  if (input.signature !== undefined) {
    if (!isPlainObject(input.signature)) {
      errors.push({ field: 'signature', message: 'Must be an object if present' });
    } else {
      if (typeof input.signature.algorithm !== 'string' || input.signature.algorithm.length === 0) {
        errors.push({
          field: 'signature.algorithm',
          message: 'Must be a non-empty string',
        });
      }
      if (typeof input.signature.value !== 'string' || input.signature.value.length === 0) {
        errors.push({
          field: 'signature.value',
          message: 'Must be a non-empty string',
        });
      }
    }
  }

  if (errors.length > 0) {
    return { valid: false, errors };
  }

  return {
    valid: true,
    errors: [],
    manifest: input as unknown as PluginManifest,
  };
}
