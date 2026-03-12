import { afterEach, describe, expect, it, vi } from 'vitest';
import { data } from '../data.js';

describe('data native fast-path API', () => {
  afterEach(() => {
    const g = globalThis as Record<string, unknown>;
    delete g['window'];
  });

  it('profiles through the reserved native channel', async () => {
    const bridge = {
      invoke: vi.fn(async () => ({
        datasetSize: 1200,
        cachedSizes: [1200],
        categorySpread: { Finance: 200 },
        regionSpread: { 'us-east': 200 },
      })),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    const result = await data.profile({ datasetSize: 1200 });

    expect(bridge.invoke).toHaveBeenCalledWith('volt:native:data.profile', { datasetSize: 1200 });
    expect(result.datasetSize).toBe(1200);
    expect(result.cachedSizes).toEqual([1200]);
  });

  it('queries through the reserved native channel', async () => {
    const bridge = {
      invoke: vi.fn(async () => ({
        datasetSize: 2400,
        iterations: 2,
        query: 'risk',
        minScore: 61,
        topN: 12,
        backendDurationMs: 8,
        filterDurationMs: 3,
        sortDurationMs: 3,
        aggregateDurationMs: 2,
        peakMatches: 480,
        totalMatchesAcrossIterations: 960,
        categoryWinners: [{ category: 'Finance', total: 10 }],
        sample: [{ id: 1, title: 'demo', category: 'Finance', region: 'us-east', score: 90, revenue: 1000, margin: 20 }],
        payloadBytes: 512,
      })),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    const result = await data.query({ datasetSize: 2400, iterations: 2, searchTerm: 'risk' });

    expect(bridge.invoke).toHaveBeenCalledWith('volt:native:data.query', {
      datasetSize: 2400,
      iterations: 2,
      searchTerm: 'risk',
    });
    expect(result.backendDurationMs).toBe(8);
    expect(result.categoryWinners[0]?.category).toBe('Finance');
  });
});
