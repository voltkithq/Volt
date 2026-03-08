export interface VoltTestLogger {
  log(message: string): void;
  warn(message: string): void;
  error(message: string): void;
}

export interface VoltTestArtifactCaptureResult {
  path: string;
  captured: boolean;
}

export interface VoltTestSuiteContext {
  repoRoot: string;
  cliEntryPath: string;
  logger: VoltTestLogger;
  timeoutMs: number;
  suiteName: string;
  attempt: number;
  artifactsDir: string;
  captureScreenshot(name?: string): Promise<VoltTestArtifactCaptureResult>;
}

export interface VoltTestSuite {
  name: string;
  timeoutMs?: number;
  run(context: VoltTestSuiteContext): Promise<void>;
}

export interface VoltTestConfig {
  timeoutMs?: number;
  retries?: number;
  artifactDir?: string;
  suites: VoltTestSuite[];
}

export interface LoadedVoltTestConfig {
  configPath: string;
  config: VoltTestConfig;
}

export interface LoadVoltTestConfigOptions {
  configPath?: string;
  strict?: boolean;
}

export interface RunSuitesOptions {
  cliEntryPath: string;
  suiteNames?: readonly string[];
  timeoutMs?: number;
  retries?: number;
  artifactDir?: string;
  captureScreenshots?: boolean;
  repoRoot?: string;
  logger?: VoltTestLogger;
}
