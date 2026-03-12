import { invoke } from './ipc.js';

export const NATIVE_DATA_PROFILE_CHANNEL = 'volt:native:data.profile';
export const NATIVE_DATA_QUERY_CHANNEL = 'volt:native:data.query';
export const NATIVE_WORKFLOW_RUN_CHANNEL = 'volt:native:workflow.run';

export async function invokeNativeFastPath<T>(
  channel: string,
  args: unknown = null,
): Promise<T> {
  return invoke<T>(channel, args);
}
