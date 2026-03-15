import type {
  AnalyticsBenchmarkOptions,
  AnalyticsBenchmarkResult,
  AnalyticsProfile,
  AnalyticsProfileOptions,
} from './types.js';
import { AnalyticsRecord, toPositiveInteger, toPseudoDuration } from './shared.js';

const datasetCache = new Map<number, AnalyticsRecord[]>();
const analyticsCategories = ['Finance', 'Security', 'Ops', 'Growth', 'Compliance', 'Platform'];
const analyticsRegions = ['us-east', 'us-west', 'emea', 'latam', 'apac'];
const analyticsOwners = ['Avery', 'Jordan', 'Morgan', 'Kai', 'Sage', 'Parker', 'Riley'];
const analyticsStatuses = ['active', 'pending', 'review', 'archived'];
const analyticsTagSets = [
  ['risk', 'delta', 'forecast'],
  ['uptime', 'queue', 'latency'],
  ['margin', 'renewal', 'pipeline'],
  ['trust', 'audit', 'policy'],
  ['urgent', 'backfill', 'priority'],
];

function getAnalyticsDataset(datasetSize: number): AnalyticsRecord[] {
  const existing = datasetCache.get(datasetSize);
  if (existing) {
    return existing;
  }

  const rows: AnalyticsRecord[] = [];
  for (let index = 0; index < datasetSize; index += 1) {
    const category = analyticsCategories[index % analyticsCategories.length];
    const region = analyticsRegions[(index * 3) % analyticsRegions.length];
    const owner = analyticsOwners[(index * 5) % analyticsOwners.length];
    const status = analyticsStatuses[(index * 7) % analyticsStatuses.length];
    const tags = analyticsTagSets[index % analyticsTagSets.length];
    const priority = (index * 17) % 100;
    const score = (index * 19 + priority * 3) % 100;
    const revenue = 900 + ((index * 97) % 12_500);
    const margin = ((index * 31) % 41) + 8;

    rows.push({
      id: index + 1,
      title: `${category} ${region} ${owner} risk pipeline ${index + 1}`,
      category,
      region,
      owner,
      status,
      priority,
      score,
      revenue,
      margin,
      tags,
      updatedAt: 1_710_000_000_000 + index * 91_000,
    });
  }

  datasetCache.set(datasetSize, rows);
  return rows;
}

function estimateAnalyticsFilterDurationMs(
  datasetSize: number,
  query: string,
  iteration: number,
): number {
  return toPseudoDuration(datasetSize, 4_500, 1, Math.floor(query.length / 6) + (iteration % 2));
}

function estimateAnalyticsSortDurationMs(
  matchCount: number,
  topN: number,
  iteration: number,
): number {
  return toPseudoDuration(matchCount + topN * 35, 900, 1, iteration % 2);
}

function estimateAnalyticsAggregateDurationMs(
  matchCount: number,
  bucketCount: number,
  iteration: number,
): number {
  return toPseudoDuration(matchCount + bucketCount * 200, 2_400, 1, iteration % 2);
}

export async function analyticsProfile(
  options: AnalyticsProfileOptions = {},
): Promise<AnalyticsProfile> {
  const datasetSize = toPositiveInteger(options.datasetSize, 24_000, 1_000, 120_000);
  const rows = getAnalyticsDataset(datasetSize);
  const categorySpread: Record<string, number> = {};
  const regionSpread: Record<string, number> = {};

  for (const row of rows) {
    categorySpread[row.category] = (categorySpread[row.category] ?? 0) + 1;
    regionSpread[row.region] = (regionSpread[row.region] ?? 0) + 1;
  }

  return {
    datasetSize: rows.length,
    cachedSizes: Array.from(datasetCache.keys()).sort((left, right) => left - right),
    categorySpread,
    regionSpread,
  };
}

export async function runAnalyticsBenchmark(
  options: AnalyticsBenchmarkOptions = {},
): Promise<AnalyticsBenchmarkResult> {
  const datasetSize = toPositiveInteger(options.datasetSize, 36_000, 1_000, 120_000);
  const iterations = toPositiveInteger(options.iterations, 6, 1, 20);
  const minScore = toPositiveInteger(options.minScore, 58, 0, 100);
  const topN = toPositiveInteger(options.topN, 18, 5, 100);
  const query =
    typeof options.searchTerm === 'string' && options.searchTerm.trim().length > 0
      ? options.searchTerm.trim().toLowerCase()
      : 'risk';
  const rows = getAnalyticsDataset(datasetSize);

  let filterDurationMs = 0;
  let sortDurationMs = 0;
  let aggregateDurationMs = 0;
  let peakMatches = 0;
  let totalMatchesAcrossIterations = 0;
  let latestTop: AnalyticsRecord[] = [];
  let latestWinners: Array<{ category: string; total: number }> = [];

  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const matches = rows.filter((row) => {
      if (row.status === 'archived' || row.score < minScore) {
        return false;
      }
      const tagBlob = row.tags.join(' ');
      const haystack = `${row.title} ${row.owner} ${row.region} ${tagBlob}`.toLowerCase();
      return haystack.includes(query);
    });
    filterDurationMs += estimateAnalyticsFilterDurationMs(datasetSize, query, iteration);
    peakMatches = Math.max(peakMatches, matches.length);
    totalMatchesAcrossIterations += matches.length;

    matches.sort((left, right) => {
      const leftWeight =
        left.score * 4 + left.margin * 3 + left.priority + Math.floor(left.revenue / 200);
      const rightWeight =
        right.score * 4 + right.margin * 3 + right.priority + Math.floor(right.revenue / 200);
      return rightWeight - leftWeight || right.updatedAt - left.updatedAt;
    });
    latestTop = matches.slice(0, topN);
    sortDurationMs += estimateAnalyticsSortDurationMs(matches.length, topN, iteration);

    const buckets = new Map<string, number>();
    for (const row of matches) {
      const bucketScore = row.score + row.margin + Math.floor(row.revenue / 500);
      buckets.set(row.category, (buckets.get(row.category) ?? 0) + bucketScore);
    }
    latestWinners = Array.from(buckets.entries())
      .map(([category, total]) => ({ category, total }))
      .sort((left, right) => right.total - left.total)
      .slice(0, 4);
    aggregateDurationMs += estimateAnalyticsAggregateDurationMs(
      matches.length,
      latestWinners.length,
      iteration,
    );
  }

  const result: AnalyticsBenchmarkResult = {
    datasetSize,
    iterations,
    query,
    minScore,
    topN,
    backendDurationMs:
      filterDurationMs +
      sortDurationMs +
      aggregateDurationMs +
      toPseudoDuration(datasetSize, 12_000, iterations),
    filterDurationMs,
    sortDurationMs,
    aggregateDurationMs,
    peakMatches,
    totalMatchesAcrossIterations,
    categoryWinners: latestWinners,
    sample: latestTop.map((row) => ({
      id: row.id,
      title: row.title,
      category: row.category,
      region: row.region,
      score: row.score,
      revenue: row.revenue,
      margin: row.margin,
    })),
    payloadBytes: 0,
  };
  result.payloadBytes = JSON.stringify(result).length;
  return result;
}
