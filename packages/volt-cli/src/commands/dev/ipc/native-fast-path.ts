import * as voltBench from '../runtime-modules/volt-bench.js';
import type { IpcResponse } from './response.js';

const DATA_PROFILE_CHANNEL = 'volt:native:data.profile';
const DATA_QUERY_CHANNEL = 'volt:native:data.query';
const WORKFLOW_RUN_CHANNEL = 'volt:native:workflow.run';

function normalizeOptions(args: unknown): Record<string, unknown> {
  if (args && typeof args === 'object' && !Array.isArray(args)) {
    return args as Record<string, unknown>;
  }
  return {};
}

export async function tryHandleNativeFastPath(
  request: { id: string; method: string; args?: unknown },
): Promise<IpcResponse | null> {
  try {
    switch (request.method) {
      case DATA_PROFILE_CHANNEL:
        return {
          id: request.id,
          result: await voltBench.analyticsProfile(normalizeOptions(request.args)),
        };
      case DATA_QUERY_CHANNEL:
        return {
          id: request.id,
          result: await voltBench.runAnalyticsBenchmark(normalizeOptions(request.args)),
        };
      case WORKFLOW_RUN_CHANNEL:
        return {
          id: request.id,
          result: await voltBench.runWorkflowBenchmark(normalizeOptions(request.args)),
        };
      default:
        return null;
    }
  } catch (error) {
    return {
      id: request.id,
      error: error instanceof Error ? error.message : String(error),
      errorCode: 'IPC_HANDLER_ERROR',
    };
  }
}
