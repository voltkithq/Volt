import { data, type DataProfile, type DataQueryOptions, type DataQueryResult } from 'voltkit/renderer';

export interface AnalyticsQueryFormState {
  datasetSize: number;
  iterations: number;
  searchTerm: string;
  minScore: number;
  topN: number;
}

export interface AnalyticsClientApi {
  profile(options: { datasetSize: number }): Promise<DataProfile>;
  query(options: DataQueryOptions): Promise<DataQueryResult>;
}

export function buildAnalyticsQueryOptions(form: AnalyticsQueryFormState): DataQueryOptions {
  return {
    datasetSize: form.datasetSize,
    iterations: form.iterations,
    searchTerm: form.searchTerm,
    minScore: form.minScore,
    topN: form.topN,
  };
}

export function formatAnalyticsStatus(
  result: Pick<DataQueryResult, 'backendDurationMs' | 'payloadBytes'>,
  roundTripMs: number,
): string {
  return `Finished. Backend ${result.backendDurationMs} ms, round trip ${roundTripMs} ms, payload ${result.payloadBytes} bytes.`;
}

export async function loadAnalyticsProfile(
  datasetSize: number,
  api: AnalyticsClientApi = data,
): Promise<DataProfile> {
  return api.profile({ datasetSize });
}

export async function runAnalyticsQuery(
  form: AnalyticsQueryFormState,
  api: AnalyticsClientApi = data,
): Promise<DataQueryResult> {
  return api.query(buildAnalyticsQueryOptions(form));
}
