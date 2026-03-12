import { listWorkflowPlugins, workflowPlugins, type WorkflowDocument } from './plugins.js';

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

const titles = ['Risk queue', 'Latency sweep', 'Renewal drift', 'Policy delta', 'Security burst', 'Audit spill'];
const bodies = [
  'urgent security queue pressure from renewal cohort',
  'latency tracking indicates policy drift in edge cluster',
  'audit pipeline shows backlog and regional imbalance',
  'renewal ops requested deeper risk review on security path',
];
const owners = ['Avery', 'Jordan', 'Morgan', 'Riley', 'Parker', 'Sage'];
const regions = ['us-east', 'us-west', 'emea', 'apac'];

function toRangeInteger(value: unknown, fallback: number, min: number, max: number): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, Math.round(numeric)));
}

function buildDocuments(batchSize: number): WorkflowDocument[] {
  const documents: WorkflowDocument[] = [];
  for (let index = 0; index < batchSize; index += 1) {
    documents.push({
      id: index + 1,
      title: `${titles[index % titles.length]} ${index + 1}`,
      body: `${bodies[index % bodies.length]} owner ${owners[index % owners.length]} region ${regions[index % regions.length]}`,
      owner: owners[(index * 3) % owners.length],
      region: regions[(index * 5) % regions.length],
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

export function runWorkflowBenchmark(options: WorkflowBenchmarkOptions = {}): WorkflowBenchmarkResult {
  const batchSize = toRangeInteger(options.batchSize, 4_500, 500, 25_000);
  const passes = toRangeInteger(options.passes, 3, 1, 8);
  const pluginNames = new Set(listWorkflowPlugins().map((plugin) => plugin.name));
  const pipeline = Array.isArray(options.pipeline) && options.pipeline.length > 0
    ? options.pipeline.filter((plugin) => pluginNames.has(plugin))
    : workflowPlugins.map((plugin) => plugin.name);
  const selectedPlugins = workflowPlugins.filter((plugin) => pipeline.includes(plugin.name));
  const documents = buildDocuments(batchSize);
  const stepTimings = new Map<string, number>();
  const startedAt = Date.now();

  for (let pass = 0; pass < passes; pass += 1) {
    for (const plugin of selectedPlugins) {
      const pluginStartedAt = Date.now();
      plugin.run(documents, pass);
      stepTimings.set(plugin.name, (stepTimings.get(plugin.name) ?? 0) + (Date.now() - pluginStartedAt));
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
    backendDurationMs: Date.now() - startedAt,
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
