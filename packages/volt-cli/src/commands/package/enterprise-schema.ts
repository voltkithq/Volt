export type EnterprisePolicyType = 'boolean' | 'text' | 'decimal' | 'enum';

export interface EnterprisePolicyEnumValue {
  id: string;
  value: string;
  displayName: string;
}

export interface EnterprisePolicyDefinition {
  id: string;
  displayName: string;
  description: string;
  registryKey: string;
  valueName: string;
  configPath: string;
  type: EnterprisePolicyType;
  defaultValue?: string | number | boolean;
  minValue?: number;
  maxValue?: number;
  enumValues?: EnterprisePolicyEnumValue[];
}

export interface EnterprisePolicySchema {
  namespace: string;
  categoryDisplayName: string;
  policies: EnterprisePolicyDefinition[];
}

export const ENTERPRISE_POLICY_SCHEMA: EnterprisePolicySchema = {
  namespace: 'Volt.Policies',
  categoryDisplayName: 'Volt Enterprise',
  policies: [
    {
      id: 'EnableDevtools',
      displayName: 'Allow DevTools',
      description: 'Enable or disable WebView developer tools across managed deployments.',
      registryKey: 'Software\\Policies\\Volt\\Runtime',
      valueName: 'EnableDevtools',
      configPath: 'devtools',
      type: 'boolean',
      defaultValue: false,
    },
    {
      id: 'RuntimePoolSize',
      displayName: 'Runtime Pool Size',
      description: 'Configure Boa runtime pool size for production builds.',
      registryKey: 'Software\\Policies\\Volt\\Runtime',
      valueName: 'RuntimePoolSize',
      configPath: 'runtime.poolSize',
      type: 'decimal',
      minValue: 2,
      maxValue: 8,
      defaultValue: 4,
    },
    {
      id: 'UpdaterEndpoint',
      displayName: 'Updater Endpoint Override',
      description: 'Set a managed updater endpoint URL for enterprise-controlled release channels.',
      registryKey: 'Software\\Policies\\Volt\\Updater',
      valueName: 'UpdaterEndpoint',
      configPath: 'updater.endpoint',
      type: 'text',
    },
    {
      id: 'UpdaterPublicKey',
      displayName: 'Updater Public Key Override',
      description: 'Set the managed updater signing public key (base64 Ed25519).',
      registryKey: 'Software\\Policies\\Volt\\Updater',
      valueName: 'UpdaterPublicKey',
      configPath: 'updater.publicKey',
      type: 'text',
    },
    {
      id: 'InstallMode',
      displayName: 'Default Install Scope',
      description: 'Choose default install scope for NSIS installers.',
      registryKey: 'Software\\Policies\\Volt\\Packaging',
      valueName: 'InstallMode',
      configPath: 'package.windows.installMode',
      type: 'enum',
      defaultValue: 'perMachine',
      enumValues: [
        {
          id: 'PerMachine',
          value: 'perMachine',
          displayName: 'Per-machine (all users)',
        },
        {
          id: 'PerUser',
          value: 'perUser',
          displayName: 'Per-user (current user only)',
        },
      ],
    },
  ],
};
