export interface AnalyticsRecord {
  id: number;
  title: string;
  category: string;
  region: string;
  owner: string;
  status: string;
  priority: number;
  score: number;
  revenue: number;
  margin: number;
  tags: string[];
  updatedAt: number;
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

const datasetCache = new Map<number, AnalyticsRecord[]>();
const categories = ['Finance', 'Security', 'Ops', 'Growth', 'Compliance', 'Platform'];
const regions = ['us-east', 'us-west', 'emea', 'latam', 'apac'];
const owners = ['Avery', 'Jordan', 'Morgan', 'Kai', 'Sage', 'Parker', 'Riley'];
const statuses = ['active', 'pending', 'review', 'archived'];
const tagSets = [
  ['risk', 'delta', 'forecast'],
  ['uptime', 'queue', 'latency'],
  ['margin', 'renewal', 'pipeline'],
  ['trust', 'audit', 'policy'],
  ['urgent', 'backfill', 'priority'],
];

function toPositiveInteger(value: unknown, fallback: number, min: number, max: number): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, Math.round(numeric)));
}

function getDataset(datasetSize: number): AnalyticsRecord[] {
  const existing = datasetCache.get(datasetSize);
  if (existing) {
    return existing;
  }

  const rows: AnalyticsRecord[] = [];
  for (let index = 0; index < datasetSize; index += 1) {
    const category = categories[index % categories.length];
    const region = regions[(index * 3) % regions.length];
    const owner = owners[(index * 5) % owners.length];
    const status = statuses[(index * 7) % statuses.length];
    const tags = tagSets[index % tagSets.length];
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

export function getAnalyticsProfile(datasetSize = 24_000): AnalyticsProfile {
  const rows = getDataset(datasetSize);
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

export function runAnalyticsBenchmark(options: AnalyticsBenchmarkOptions = {}): AnalyticsBenchmarkResult {
  const datasetSize = toPositiveInteger(options.datasetSize, 36_000, 1_000, 120_000);
  const iterations = toPositiveInteger(options.iterations, 6, 1, 20);
  const minScore = toPositiveInteger(options.minScore, 58, 0, 100);
  const topN = toPositiveInteger(options.topN, 18, 5, 100);
  const query = typeof options.searchTerm === 'string' && options.searchTerm.trim().length > 0
    ? options.searchTerm.trim().toLowerCase()
    : 'risk';
  const rows = getDataset(datasetSize);

  let filterDurationMs = 0;
  let sortDurationMs = 0;
  let aggregateDurationMs = 0;
  let peakMatches = 0;
  let totalMatchesAcrossIterations = 0;
  let latestTop: AnalyticsRecord[] = [];
  let latestWinners: Array<{ category: string; total: number }> = [];

  const benchmarkStartedAt = Date.now();

  for (let iteration = 0; iteration < iterations; iteration += 1) {
    const filterStartedAt = Date.now();
    const matches = rows.filter((row) => {
      if (row.status === 'archived') {
        return false;
      }
      if (row.score < minScore) {
        return false;
      }
      const tagBlob = row.tags.join(' ');
      const haystack = `${row.title} ${row.owner} ${row.region} ${tagBlob}`.toLowerCase();
      return haystack.includes(query);
    });
    filterDurationMs += Date.now() - filterStartedAt;
    peakMatches = Math.max(peakMatches, matches.length);
    totalMatchesAcrossIterations += matches.length;

    const sortStartedAt = Date.now();
    matches.sort((left, right) => {
      const leftWeight = left.score * 4 + left.margin * 3 + left.priority + Math.floor(left.revenue / 200);
      const rightWeight = right.score * 4 + right.margin * 3 + right.priority + Math.floor(right.revenue / 200);
      return rightWeight - leftWeight || right.updatedAt - left.updatedAt;
    });
    latestTop = matches.slice(0, topN);
    sortDurationMs += Date.now() - sortStartedAt;

    const aggregateStartedAt = Date.now();
    const buckets = new Map<string, number>();
    for (const row of matches) {
      const bucketScore = row.score + row.margin + Math.floor(row.revenue / 500);
      buckets.set(row.category, (buckets.get(row.category) ?? 0) + bucketScore);
    }
    latestWinners = Array.from(buckets.entries())
      .map(([category, total]) => ({ category, total }))
      .sort((left, right) => right.total - left.total)
      .slice(0, 4);
    aggregateDurationMs += Date.now() - aggregateStartedAt;
  }

  const result: AnalyticsBenchmarkResult = {
    datasetSize,
    iterations,
    query,
    minScore,
    topN,
    backendDurationMs: Date.now() - benchmarkStartedAt,
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
