/** Configuration for the Volt application, typically defined in volt.config.ts. */
export interface VoltConfig {
  /** Display name of the application. */
  name: string;
  /** Application version (semver). */
  version?: string;
  /** Window configuration. */
  window?: WindowOptions;
  /** Declared permissions for native API access. */
  permissions?: Permission[];
  /** Build configuration. */
  build?: BuildConfig;
  /** Backend entrypoint bundled into the standalone runtime (e.g. './src/backend.ts'). */
  backend?: string;
  /** Packaging configuration for platform installers. */
  package?: PackageConfig;
  /** Auto-updater configuration. */
  updater?: UpdaterConfig;
  /** QuickJS runtime pool configuration for production builds. */
  runtime?: RuntimeConfig;
  /** Legacy alias for runtime.poolSize. Prefer using runtime.poolSize. */
  runtimePoolSize?: number;
  /** Whether to enable devtools. Defaults to true in dev, false in production. */
  devtools?: boolean;
}

/** Window creation options, compatible with Electron's BrowserWindow subset. */
export interface WindowOptions {
  /** Window width in pixels. Default: 800. */
  width?: number;
  /** Window height in pixels. Default: 600. */
  height?: number;
  /** Minimum window width. */
  minWidth?: number;
  /** Minimum window height. */
  minHeight?: number;
  /** Maximum window width. */
  maxWidth?: number;
  /** Maximum window height. */
  maxHeight?: number;
  /** Window title. */
  title?: string;
  /** Whether the window is resizable. Default: true. */
  resizable?: boolean;
  /** Whether to show window decorations. Default: true. */
  decorations?: boolean;
  /** Whether window is transparent. Default: false. */
  transparent?: boolean;
  /** Whether window is always on top. Default: false. */
  alwaysOnTop?: boolean;
  /** Whether window starts maximized. Default: false. */
  maximized?: boolean;
  /** Whether WebView devtools should be enabled for this window. */
  devtools?: boolean;
  /** Initial X position. */
  x?: number;
  /** Initial Y position. */
  y?: number;
  /** Path to the window icon (PNG). Displayed in the title bar and taskbar. */
  icon?: string;
}

/** Capability-based permissions that must be declared in volt.config.ts. */
export type Permission =
  | 'clipboard'
  | 'notification'
  | 'dialog'
  | 'fs'
  | 'db'
  | 'menu'
  | 'shell'
  | 'http'
  | 'globalShortcut'
  | 'tray'
  | 'secureStorage';

/** Build output configuration. */
export interface BuildConfig {
  /** Output directory for built assets. Default: 'dist'. */
  outDir?: string;
}

/** Runtime execution configuration. */
export interface RuntimeConfig {
  /** Number of QuickJS runtimes in the IPC pool. Clamped to [2, 4] by the runner. */
  poolSize?: number;
}

/** Packaging configuration for platform-specific installers. */
export interface PackageConfig {
  /** Application identifier (e.g., 'com.example.myapp'). */
  identifier: string;
  /** Path to the application icon. */
  icon?: string;
  /** Application categories (for Linux .desktop files). */
  categories?: string[];
  /** Windows-specific packaging settings (NSIS/MSIX/install mode). */
  windows?: PackageWindowsConfig;
  /** Enterprise packaging outputs (ADMX/docs bundle generation). */
  enterprise?: PackageEnterpriseConfig;
  /** Code signing configuration. Opt-in - omit to skip signing. */
  signing?: SigningConfig;
}

/** Default Windows installation scope for generated installers. */
export type WindowsInstallMode = 'perMachine' | 'perUser';

/** MSIX metadata overrides for Windows package identity and display fields. */
export interface PackageWindowsMsixConfig {
  /** Package identity name (must remain stable for updates). */
  identityName?: string;
  /** Publisher subject, usually a CN value (for example "CN=Contoso"). */
  publisher?: string;
  /** Publisher display name shown in Windows package metadata. */
  publisherDisplayName?: string;
  /** Display name shown by Windows for the installed package. */
  displayName?: string;
  /** Description shown in Windows package metadata. */
  description?: string;
}

/** Windows packaging settings for NSIS and MSIX outputs. */
export interface PackageWindowsConfig {
  /** Default install scope for NSIS packaging and enterprise policy output. */
  installMode?: WindowsInstallMode;
  /** Force /ALLUSERS=1 when installer runs silently (per-machine mode only). */
  silentAllUsers?: boolean;
  /** Optional MSIX identity and metadata overrides. */
  msix?: PackageWindowsMsixConfig;
}

/** Enterprise packaging controls for policy/docs deployment outputs. */
export interface PackageEnterpriseConfig {
  /** Emit ADMX/ADML policy templates and policy-values JSON. Default: true. */
  generateAdmx?: boolean;
  /** Emit deployment docs and PowerShell install scripts. Default: true. */
  includeDocsBundle?: boolean;
}
/** Code signing configuration for macOS and Windows. */
export interface SigningConfig {
  /** macOS code signing and notarization. */
  macOS?: MacOSSigningConfig;
  /** Windows code signing (Authenticode). */
  windows?: WindowsSigningConfig;
}

/** macOS code signing configuration.
 *  Sensitive values (certificates, passwords) should be set via environment
 *  variables, not in this config. See docs/api/signing.md. */
export interface MacOSSigningConfig {
  /** Signing identity (e.g., "Developer ID Application: Name (TEAMID)").
   *  Overridden by VOLT_MACOS_SIGNING_IDENTITY env var. */
  identity?: string;
  /** Path to entitlements .plist file. */
  entitlements?: string;
  /** Whether to notarize after signing. Default: true when identity is set. */
  notarize?: boolean;
  /** Apple Developer Team ID for notarization.
   *  Overridden by VOLT_APPLE_TEAM_ID env var. */
  teamId?: string;
}

/** Windows code signing configuration.
 *  Certificate password should be set via VOLT_WIN_CERTIFICATE_PASSWORD env var. */
export interface WindowsSigningConfig {
  /** Signing backend provider. Default: "local". */
  provider?: 'local' | 'azureTrustedSigning' | 'digicertKeyLocker';
  /** Path to certificate file (.pfx/.p12).
   *  Overridden by VOLT_WIN_CERTIFICATE env var. */
  certificate?: string;
  /** Digest algorithm. Default: "sha256". */
  digestAlgorithm?: string;
  /** RFC 3161 timestamp server URL. Default: "http://timestamp.digicert.com". */
  timestampUrl?: string;
  /** Azure Trusted Signing provider settings (when provider = "azureTrustedSigning"). */
  azureTrustedSigning?: AzureTrustedSigningConfig;
  /** DigiCert KeyLocker provider settings (when provider = "digicertKeyLocker"). */
  digicertKeyLocker?: DigiCertKeyLockerConfig;
}

/** Azure Trusted Signing provider configuration for Windows Authenticode. */
export interface AzureTrustedSigningConfig {
  /** Path to Azure.CodeSigning.Dlib.dll. Overridden by VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH. */
  dlibPath?: string;
  /** Path to Azure Trusted Signing metadata JSON. Overridden by VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH. */
  metadataPath?: string;
  /** Optional account endpoint. Overridden by VOLT_AZURE_TRUSTED_SIGNING_ENDPOINT. */
  endpoint?: string;
  /** Optional account name. Overridden by VOLT_AZURE_TRUSTED_SIGNING_ACCOUNT_NAME. */
  accountName?: string;
  /** Optional certificate profile name. Overridden by VOLT_AZURE_TRUSTED_SIGNING_CERT_PROFILE. */
  certificateProfileName?: string;
  /** Optional correlation ID for provider-side traceability. Overridden by VOLT_AZURE_TRUSTED_SIGNING_CORRELATION_ID. */
  correlationId?: string;
}

/** DigiCert KeyLocker provider configuration for Windows Authenticode. */
export interface DigiCertKeyLockerConfig {
  /** Key pair alias in DigiCert KeyLocker. Overridden by VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS. */
  keypairAlias?: string;
  /** Optional cert fingerprint. Overridden by VOLT_DIGICERT_KEYLOCKER_CERT_FINGERPRINT. */
  certificateFingerprint?: string;
  /** Optional path to smctl executable. Overridden by VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH. */
  smctlPath?: string;
  /** Optional RFC 3161 timestamp URL. Overridden by VOLT_DIGICERT_KEYLOCKER_TIMESTAMP_URL. */
  timestampUrl?: string;
}

/** Auto-updater configuration. */
export interface UpdaterConfig {
  /** URL endpoint to check for updates. */
  endpoint: string;
  /** Ed25519 public key for signature verification (base64). */
  publicKey: string;
  /** Optional telemetry controls for updater lifecycle instrumentation. */
  telemetry?: UpdaterTelemetryConfig;
}

/** Updater telemetry controls (disabled by default). */
export interface UpdaterTelemetryConfig {
  /** Enables updater telemetry events and sink emission. Default: false. */
  enabled?: boolean;
  /** Sink adapter identifier. `none` keeps telemetry in no-op mode. */
  sink?: 'none' | 'stdout';
}

/** Define a Volt configuration with full type checking. */
export function defineConfig(config: VoltConfig): VoltConfig {
  return config;
}
