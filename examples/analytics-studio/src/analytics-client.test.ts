import { describe, expect, it, vi } from 'vitest';
import {
  buildAnalyticsQueryOptions,
  formatAnalyticsStatus,
  loadAnalyticsProfile,
  runAnalyticsQuery,
  type AnalyticsClientApi,
  type AnalyticsQueryFormState,
} from './analytics-client.js';

function createApi(): AnalyticsClientApi {
  return {
    profile: vi.fn(async () => ({
      datasetSize: 1200,
      cachedSizes: [1200],
      categorySpread: { Finance: 200 },
      regionSpread: { 'us-east': 200 },
    })),
    query: vi.fn(async () => ({
      datasetSize: 2400,
      iterations: 2,
      query: 'risk',
      minScore: 61,
      topN: 12,
      backendDurationMs: 11,
      filterDurationMs: 4,
      sortDurationMs: 4,
      aggregateDurationMs: 3,
      peakMatches: 320,
      totalMatchesAcrossIterations: 640,
      categoryWinners: [{ category: 'Finance', total: 11 }],
      sample: [],
      payloadBytes: 444,
    })),
  };
}

describe('analytics client', () => {
  it('builds query options from form state', () => {
    const form: AnalyticsQueryFormState = {
      datasetSize: 2400,
      iterations: 2,
      searchTerm: 'risk',
      minScore: 61,
      topN: 12,
    };

    expect(buildAnalyticsQueryOptions(form)).toEqual({
      datasetSize: 2400,
      iterations: 2,
      searchTerm: 'risk',
      minScore: 61,
      topN: 12,
    });
  });

  it('loads a profile through the public data API contract', async () => {
    const api = createApi();

    const result = await loadAnalyticsProfile(1200, api);

    expect(api.profile).toHaveBeenCalledWith({ datasetSize: 1200 });
    expect(result.datasetSize).toBe(1200);
  });

  it('runs queries through the public data API contract', async () => {
    const api = createApi();

    const result = await runAnalyticsQuery({
      datasetSize: 2400,
      iterations: 2,
      searchTerm: 'risk',
      minScore: 61,
      topN: 12,
    }, api);

    expect(api.query).toHaveBeenCalledWith({
      datasetSize: 2400,
      iterations: 2,
      searchTerm: 'risk',
      minScore: 61,
      topN: 12,
    });
    expect(result.backendDurationMs).toBe(11);
  });

  it('formats the renderer status line from returned metrics', () => {
    expect(formatAnalyticsStatus({ backendDurationMs: 18, payloadBytes: 512 }, 21)).toBe(
      'Finished. Backend 18 ms, round trip 21 ms, payload 512 bytes.',
    );
  });
});
