import { invoke } from './ipc.js';
import {
  NATIVE_WORKFLOW_RUN_CHANNEL,
  invokeNativeFastPath,
} from './native-fast-path.js';

const WORKFLOW_PLUGINS_CHANNEL = 'workflow:plugins';

export interface WorkflowPluginInfo {
  name: string;
  label: string;
  description: string;
}

export interface WorkflowRunOptions {
  batchSize?: number;
  passes?: number;
  pipeline?: string[];
}

export interface WorkflowRunResult {
  batchSize: number;
  passes: number;
  pipeline: string[];
  backendDurationMs: number;
  stepTimings: Array<{ plugin: string; durationMs: number }>;
  routeDistribution: Record<string, number>;
  averagePriority: number;
  digestSample: string[];
  payloadBytes: number;
}

async function listPlugins(): Promise<WorkflowPluginInfo[]> {
  return invoke<WorkflowPluginInfo[]>(WORKFLOW_PLUGINS_CHANNEL);
}

async function run(options: WorkflowRunOptions = {}): Promise<WorkflowRunResult> {
  return invokeNativeFastPath<WorkflowRunResult>(NATIVE_WORKFLOW_RUN_CHANNEL, options);
}

export const workflow = {
  listPlugins,
  run,
};
