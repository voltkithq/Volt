declare module 'volt:bench' {
  export interface AnalyticsProfileOptions {
    datasetSize?: number;
  }

  export interface AnalyticsProfile {
    datasetSize: number;
    cachedSizes: number[];
    categorySpread: Record<string, number>;
    regionSpread: Record<string, number>;
  }

  export interface AnalyticsBenchmarkOptions {
    datasetSize?: number;
    iterations?: number;
    searchTerm?: string;
    minScore?: number;
    topN?: number;
  }

  export interface AnalyticsBenchmarkResult {
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

  export interface WorkflowBenchmarkOptions {
    batchSize?: number;
    passes?: number;
    pipeline?: string[];
  }

  export interface WorkflowBenchmarkResult {
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

  export function analyticsProfile(options?: AnalyticsProfileOptions): Promise<AnalyticsProfile>;
  export function runAnalyticsBenchmark(
    options?: AnalyticsBenchmarkOptions,
  ): Promise<AnalyticsBenchmarkResult>;
  export function runWorkflowBenchmark(
    options?: WorkflowBenchmarkOptions,
  ): Promise<WorkflowBenchmarkResult>;
}
