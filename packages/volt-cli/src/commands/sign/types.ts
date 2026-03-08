export type SignSetupPlatform = 'darwin' | 'win32' | 'all';

export type SignSetupWindowsProvider = 'local' | 'azureTrustedSigning' | 'digicertKeyLocker';

export interface SignSetupOptions {
  platform?: string;
  windowsProvider?: string;
  output?: string;
  force?: boolean;
  print?: boolean;
  printOnly?: boolean;
}

export interface SignSetupContext {
  platform: SignSetupPlatform;
  windowsProvider: SignSetupWindowsProvider;
}
