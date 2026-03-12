export interface AnalyticsProfileOptions {
  datasetSize?: number;
}

interface AnalyticsRecord {
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

interface WorkflowDocument {
  id: number;
  title: string;
  body: string;
  owner: string;
  region: string;
  normalized: string;
  tokens: string[];
  tags: string[];
  priority: number;
  riskScore: number;
  route: string;
  digest: string;
}

interface WorkflowPluginDefinition {
  name: string;
  weight: number;
  run(documents: WorkflowDocument[], pass: number): void;
}

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

const workflowTitles = ['Risk queue', 'Latency sweep', 'Renewal drift', 'Policy delta', 'Security burst', 'Audit spill'];
const workflowBodies = [
  'urgent security queue pressure from renewal cohort',
  'latency tracking indicates policy drift in edge cluster',
  'audit pipeline shows backlog and regional imbalance',
  'renewal ops requested deeper risk review on security path',
];
const workflowOwners = ['Avery', 'Jordan', 'Morgan', 'Riley', 'Parker', 'Sage'];
const workflowRegions = ['us-east', 'us-west', 'emea', 'apac'];
const cleanupRegex = /[^a-z0-9\s]+/g;
const whitespaceRegex = /\s+/g;

function toPositiveInteger(value: unknown, fallback: number, min: number, max: number): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, Math.round(numeric)));
}

function toPseudoDuration(units: number, divisor: number, baseline = 1, modifier = 0): number {
  return Math.max(1, Math.round(units / divisor) + baseline + modifier);
}

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

function estimateAnalyticsFilterDurationMs(datasetSize: number, query: string, iteration: number): number {
  return toPseudoDuration(datasetSize, 4_500, 1, Math.floor(query.length / 6) + (iteration % 2));
}

function estimateAnalyticsSortDurationMs(matchCount: number, topN: number, iteration: number): number {
  return toPseudoDuration(matchCount + (topN * 35), 900, 1, iteration % 2);
}

function estimateAnalyticsAggregateDurationMs(matchCount: number, bucketCount: number, iteration: number): number {
  return toPseudoDuration(matchCount + (bucketCount * 200), 2_400, 1, iteration % 2);
}

function buildWorkflowDocuments(batchSize: number): WorkflowDocument[] {
  const documents: WorkflowDocument[] = [];
  for (let index = 0; index < batchSize; index += 1) {
    documents.push({
      id: index + 1,
      title: `${workflowTitles[index % workflowTitles.length]} ${index + 1}`,
      body: `${workflowBodies[index % workflowBodies.length]} owner ${workflowOwners[index % workflowOwners.length]} region ${workflowRegions[index % workflowRegions.length]}`,
      owner: workflowOwners[(index * 3) % workflowOwners.length],
      region: workflowRegions[(index * 5) % workflowRegions.length],
      normalized: '',
      tokens: [],
      tags: [],
      priority: 0,
      riskScore: 0,
      route: 'pending',
      digest: '',
    });
  }
  return documents;
}

function normalizeText(documents: WorkflowDocument[], pass: number): void {
  for (const document of documents) {
    const merged = `${document.title} ${document.body} ${document.owner} ${document.region}`.toLowerCase();
    document.normalized = merged.replace(cleanupRegex, ' ').replace(whitespaceRegex, ' ').trim();
    if (pass % 2 === 1) {
      document.normalized = `${document.normalized} p${pass}`;
    }
  }
}

function extractSignals(documents: WorkflowDocument[], pass: number): void {
  const heatTerms = ['urgent', 'queue', 'risk', 'renewal', 'latency', 'security', 'audit'];
  for (const document of documents) {
    const tokens = document.normalized.split(' ');
    document.tokens = tokens;
    const tags = new Set(document.tags);
    for (const term of heatTerms) {
      if (tokens.includes(term)) {
        tags.add(term);
      }
    }
    if ((document.id + pass) % 4 === 0) {
      tags.add('burst');
    }
    document.tags = Array.from(tags).slice(0, 8);
  }
}

function scorePriority(documents: WorkflowDocument[], pass: number): void {
  for (const document of documents) {
    let score = 0;
    score += document.tokens.length * 2;
    score += document.tags.length * 5;
    score += document.region === 'emea' ? 7 : 3;
    score += document.owner.length;
    if (document.normalized.includes('security')) {
      score += 18;
    }
    if (document.normalized.includes('latency')) {
      score += 12;
    }
    document.riskScore = score + pass * 3;
    document.priority = Math.min(100, score + (document.id % 19));
  }
}

function routeQueues(documents: WorkflowDocument[], pass: number): void {
  for (const document of documents) {
    if (document.riskScore >= 70) {
      document.route = pass % 2 === 0 ? 'rapid-response' : 'incident-review';
      continue;
    }
    if (document.priority >= 55) {
      document.route = 'priority-backlog';
      continue;
    }
    document.route = document.region === 'apac' ? 'regional-apac' : 'steady-state';
  }
}

function buildDigests(documents: WorkflowDocument[], pass: number): void {
  for (const document of documents) {
    const headline = document.tokens.slice(0, 5).join(' ');
    const tags = document.tags.join(', ');
    document.digest = `${document.route} | ${headline} | ${tags} | p${pass}`;
  }
}

const workflowPlugins: WorkflowPluginDefinition[] = [
  {
    name: 'normalizeText',
    weight: 1.2,
    run: normalizeText,
  },
  {
    name: 'extractSignals',
    weight: 1.5,
    run: extractSignals,
  },
  {
    name: 'scorePriority',
    weight: 1.3,
    run: scorePriority,
  },
  {
    name: 'routeQueues',
    weight: 0.9,
    run: routeQueues,
  },
  {
    name: 'buildDigests',
    weight: 1.1,
    run: buildDigests,
  },
];

function estimateWorkflowStepDurationMs(batchSize: number, pluginWeight: number, pass: number): number {
  return toPseudoDuration(batchSize * pluginWeight, 3_200, 1, pass % 2);
}

export async function analyticsProfile(options: AnalyticsProfileOptions = {}): Promise<AnalyticsProfile> {
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

export async function runAnalyticsBenchmark(options: AnalyticsBenchmarkOptions = {}): Promise<AnalyticsBenchmarkResult> {
  const datasetSize = toPositiveInteger(options.datasetSize, 36_000, 1_000, 120_000);
  const iterations = toPositiveInteger(options.iterations, 6, 1, 20);
  const minScore = toPositiveInteger(options.minScore, 58, 0, 100);
  const topN = toPositiveInteger(options.topN, 18, 5, 100);
  const query = typeof options.searchTerm === 'string' && options.searchTerm.trim().length > 0
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
    filterDurationMs += estimateAnalyticsFilterDurationMs(datasetSize, query, iteration);
    peakMatches = Math.max(peakMatches, matches.length);
    totalMatchesAcrossIterations += matches.length;

    matches.sort((left, right) => {
      const leftWeight = left.score * 4 + left.margin * 3 + left.priority + Math.floor(left.revenue / 200);
      const rightWeight = right.score * 4 + right.margin * 3 + right.priority + Math.floor(right.revenue / 200);
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
    aggregateDurationMs += estimateAnalyticsAggregateDurationMs(matches.length, latestWinners.length, iteration);
  }

  const result: AnalyticsBenchmarkResult = {
    datasetSize,
    iterations,
    query,
    minScore,
    topN,
    backendDurationMs: filterDurationMs + sortDurationMs + aggregateDurationMs + toPseudoDuration(datasetSize, 12_000, iterations),
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

export async function runWorkflowBenchmark(options: WorkflowBenchmarkOptions = {}): Promise<WorkflowBenchmarkResult> {
  const batchSize = toPositiveInteger(options.batchSize, 4_500, 500, 25_000);
  const passes = toPositiveInteger(options.passes, 3, 1, 8);
  const pluginNames = new Set(workflowPlugins.map((plugin) => plugin.name));
  const pipeline = Array.isArray(options.pipeline) && options.pipeline.length > 0
    ? options.pipeline.filter((plugin) => pluginNames.has(plugin))
    : workflowPlugins.map((plugin) => plugin.name);
  const selectedPlugins = workflowPlugins.filter((plugin) => pipeline.includes(plugin.name));
  const documents = buildWorkflowDocuments(batchSize);
  const stepTimings = new Map<string, number>();

  for (let pass = 0; pass < passes; pass += 1) {
    for (const plugin of selectedPlugins) {
      plugin.run(documents, pass);
      stepTimings.set(
        plugin.name,
        (stepTimings.get(plugin.name) ?? 0) + estimateWorkflowStepDurationMs(batchSize, plugin.weight, pass),
      );
    }
  }

  const routeDistribution: Record<string, number> = {};
  let totalPriority = 0;
  for (const document of documents) {
    totalPriority += document.priority;
    routeDistribution[document.route] = (routeDistribution[document.route] ?? 0) + 1;
  }

  const result: WorkflowBenchmarkResult = {
    batchSize,
    passes,
    pipeline,
    backendDurationMs: Array.from(stepTimings.values()).reduce((total, value) => total + value, 0)
      + toPseudoDuration(batchSize, 6_000, selectedPlugins.length),
    stepTimings: Array.from(stepTimings.entries()).map(([plugin, durationMs]) => ({
      plugin,
      durationMs,
    })),
    routeDistribution,
    averagePriority: documents.length > 0 ? Number((totalPriority / documents.length).toFixed(2)) : 0,
    digestSample: documents.slice(0, 6).map((document) => document.digest),
    payloadBytes: 0,
  };
  result.payloadBytes = JSON.stringify(result).length;
  return result;
}
