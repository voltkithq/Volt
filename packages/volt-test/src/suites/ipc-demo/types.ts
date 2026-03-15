import type { AutomationEvent } from '../../drivers/index.js';

export interface IpcDemoSmokeSuiteOptions {
  name?: string;
  projectDir?: string;
  timeoutMs?: number;
}

export interface IpcDemoSmokePayload {
  ok: boolean;
  ping: { pong: number };
  echo: { message: string; sentAt: string };
  compute: { sum: number; product: number };
  nativeSetup: unknown;
  status: unknown;
  dbList: unknown;
  events: AutomationEvent[];
}
