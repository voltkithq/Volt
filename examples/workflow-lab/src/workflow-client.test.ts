import { describe, expect, it, vi } from 'vitest';
import {
  collectSelectedPipeline,
  formatWorkflowStatus,
  loadWorkflowPlugins,
  runWorkflowPipeline,
  type WorkflowClientApi,
} from './workflow-client.js';

function createApi(): WorkflowClientApi {
  return {
    listPlugins: vi.fn(async () => ([
      {
        name: 'normalizeText',
        label: 'Normalize Text',
        description: 'Lowercases and cleans the document payload.',
      },
    ])),
    run: vi.fn(async () => ({
      batchSize: 1500,
      passes: 2,
      pipeline: ['normalizeText', 'buildDigests'],
      backendDurationMs: 25,
      stepTimings: [],
      routeDistribution: { 'steady-state': 10 },
      averagePriority: 62.2,
      digestSample: ['steady-state | queue | urgent'],
      payloadBytes: 512,
    })),
  };
}

describe('workflow client', () => {
  it('collects selected plugin names without empty values', () => {
    expect(collectSelectedPipeline([
      { checked: true, pluginName: 'normalizeText' },
      { checked: false, pluginName: 'routeQueues' },
      { checked: true, pluginName: undefined },
      { checked: true, pluginName: 'buildDigests' },
    ])).toEqual(['normalizeText', 'buildDigests']);
  });

  it('loads plugins through the public workflow API', async () => {
    const api = createApi();

    const plugins = await loadWorkflowPlugins(api);

    expect(api.listPlugins).toHaveBeenCalledTimes(1);
    expect(plugins[0]?.name).toBe('normalizeText');
  });

  it('runs the selected pipeline through the public workflow API', async () => {
    const api = createApi();

    const result = await runWorkflowPipeline({
      batchSize: 1500,
      passes: 2,
      pipeline: ['normalizeText', 'buildDigests'],
    }, api);

    expect(api.run).toHaveBeenCalledWith({
      batchSize: 1500,
      passes: 2,
      pipeline: ['normalizeText', 'buildDigests'],
    });
    expect(result.backendDurationMs).toBe(25);
  });

  it('formats the workflow completion message from result data', () => {
    expect(formatWorkflowStatus({
      backendDurationMs: 25,
      pipeline: ['normalizeText', 'buildDigests'],
    })).toBe('Workflow complete in 25 ms across 2 plugins.');
  });
});
