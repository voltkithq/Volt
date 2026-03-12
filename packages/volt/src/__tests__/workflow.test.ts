import { afterEach, describe, expect, it, vi } from 'vitest';
import { workflow } from '../workflow.js';

describe('workflow native fast-path API', () => {
  afterEach(() => {
    const g = globalThis as Record<string, unknown>;
    delete g['window'];
  });

  it('lists plugins through the public workflow channel', async () => {
    const bridge = {
      invoke: vi.fn(async () => ([
        {
          name: 'normalizeText',
          label: 'Normalize Text',
          description: 'Lowercases and cleans the document payload.',
        },
      ])),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    const result = await workflow.listPlugins();

    expect(bridge.invoke).toHaveBeenCalledWith('workflow:plugins', null);
    expect(result).toEqual([
      {
        name: 'normalizeText',
        label: 'Normalize Text',
        description: 'Lowercases and cleans the document payload.',
      },
    ]);
  });

  it('runs through the reserved native channel', async () => {
    const bridge = {
      invoke: vi.fn(async () => ({
        batchSize: 1800,
        passes: 2,
        pipeline: ['normalizeText', 'buildDigests'],
        backendDurationMs: 22,
        stepTimings: [
          { plugin: 'normalizeText', durationMs: 10 },
          { plugin: 'buildDigests', durationMs: 12 },
        ],
        routeDistribution: { 'steady-state': 1200 },
        averagePriority: 62.4,
        digestSample: ['steady-state | risk queue | urgent | p1'],
        payloadBytes: 444,
      })),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    const result = await workflow.run({
      batchSize: 1800,
      passes: 2,
      pipeline: ['normalizeText', 'buildDigests'],
    });

    expect(bridge.invoke).toHaveBeenCalledWith('volt:native:workflow.run', {
      batchSize: 1800,
      passes: 2,
      pipeline: ['normalizeText', 'buildDigests'],
    });
    expect(result.backendDurationMs).toBe(22);
    expect(result.pipeline).toEqual(['normalizeText', 'buildDigests']);
  });
});
