import {
  NATIVE_DATA_PROFILE_CHANNEL,
  NATIVE_DATA_QUERY_CHANNEL,
  invokeNativeFastPath,
} from './native-fast-path.js';

export interface DataProfileOptions {
  datasetSize?: number;
}

export interface DataProfile {
  datasetSize: number;
  cachedSizes: number[];
  categorySpread: Record<string, number>;
  regionSpread: Record<string, number>;
}

export interface DataQueryOptions {
  datasetSize?: number;
  iterations?: number;
  searchTerm?: string;
  minScore?: number;
  topN?: number;
}

export interface DataQueryResult {
  datasetSize: number;
  iterations: number;
  query: string;
  minScore: number;
  topN: number;
  backendDurationMs: number;
  filterDurationMs: number;
  sortDurationMs: number;
  aggregateDurationMs: number;
  peakMatches: number;
  totalMatchesAcrossIterations: number;
  categoryWinners: Array<{ category: string; total: number }>;
  sample: Array<{
    id: number;
    title: string;
    category: string;
    region: string;
    score: number;
    revenue: number;
    margin: number;
  }>;
  payloadBytes: number;
}

async function profile(options: DataProfileOptions = {}): Promise<DataProfile> {
  return invokeNativeFastPath<DataProfile>(NATIVE_DATA_PROFILE_CHANNEL, options);
}

async function query(options: DataQueryOptions = {}): Promise<DataQueryResult> {
  return invokeNativeFastPath<DataQueryResult>(NATIVE_DATA_QUERY_CHANNEL, options);
}

export const data = {
  profile,
  query,
};
