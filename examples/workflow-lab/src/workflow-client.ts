import {
  workflow,
  type WorkflowPluginInfo,
  type WorkflowRunOptions,
  type WorkflowRunResult,
} from 'voltkit/renderer';

export interface WorkflowFormState {
  batchSize: number;
  passes: number;
  pipeline: string[];
}

export interface WorkflowClientApi {
  listPlugins(): Promise<WorkflowPluginInfo[]>;
  run(options: WorkflowRunOptions): Promise<WorkflowRunResult>;
}

export function collectSelectedPipeline(
  inputs: ReadonlyArray<{ checked: boolean; pluginName?: string | undefined }>,
): string[] {
  return inputs
    .filter((input) => input.checked)
    .map((input) => input.pluginName ?? '')
    .filter((value) => value.length > 0);
}

export function formatWorkflowStatus(
  result: Pick<WorkflowRunResult, 'backendDurationMs' | 'pipeline'>,
): string {
  return `Workflow complete in ${result.backendDurationMs} ms across ${result.pipeline.length} plugins.`;
}

export async function loadWorkflowPlugins(
  api: WorkflowClientApi = workflow,
): Promise<WorkflowPluginInfo[]> {
  return api.listPlugins();
}

export async function runWorkflowPipeline(
  form: WorkflowFormState,
  api: WorkflowClientApi = workflow,
): Promise<WorkflowRunResult> {
  return api.run({
    batchSize: form.batchSize,
    passes: form.passes,
    pipeline: form.pipeline,
  });
}
