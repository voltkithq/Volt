import type { WorkflowBenchmarkOptions, WorkflowBenchmarkResult } from './types.js';
import {
  WorkflowDocument,
  WorkflowPluginDefinition,
  normalizeWorkflowText,
  toPositiveInteger,
  toPseudoDuration,
} from './shared.js';

const workflowTitles = [
  'Risk queue',
  'Latency sweep',
  'Renewal drift',
  'Policy delta',
  'Security burst',
  'Audit spill',
];
const workflowBodies = [
  'urgent security queue pressure from renewal cohort',
  'latency tracking indicates policy drift in edge cluster',
  'audit pipeline shows backlog and regional imbalance',
  'renewal ops requested deeper risk review on security path',
];
const workflowOwners = ['Avery', 'Jordan', 'Morgan', 'Riley', 'Parker', 'Sage'];
const workflowRegions = ['us-east', 'us-west', 'emea', 'apac'];

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
    const merged =
      `${document.title} ${document.body} ${document.owner} ${document.region}`.toLowerCase();
    document.normalized = normalizeWorkflowText(merged);
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
  { name: 'normalizeText', weight: 1.2, run: normalizeText },
  { name: 'extractSignals', weight: 1.5, run: extractSignals },
  { name: 'scorePriority', weight: 1.3, run: scorePriority },
  { name: 'routeQueues', weight: 0.9, run: routeQueues },
  { name: 'buildDigests', weight: 1.1, run: buildDigests },
];

function estimateWorkflowStepDurationMs(
  batchSize: number,
  pluginWeight: number,
  pass: number,
): number {
  return toPseudoDuration(batchSize * pluginWeight, 3_200, 1, pass % 2);
}

export async function runWorkflowBenchmark(
  options: WorkflowBenchmarkOptions = {},
): Promise<WorkflowBenchmarkResult> {
  const batchSize = toPositiveInteger(options.batchSize, 4_500, 500, 25_000);
  const passes = toPositiveInteger(options.passes, 3, 1, 8);
  const pluginNames = new Set(workflowPlugins.map((plugin) => plugin.name));
  const pipeline =
    Array.isArray(options.pipeline) && options.pipeline.length > 0
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
        (stepTimings.get(plugin.name) ?? 0) +
          estimateWorkflowStepDurationMs(batchSize, plugin.weight, pass),
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
    backendDurationMs:
      Array.from(stepTimings.values()).reduce((total, value) => total + value, 0) +
      toPseudoDuration(batchSize, 6_000, selectedPlugins.length),
    stepTimings: Array.from(stepTimings.entries()).map(([plugin, durationMs]) => ({
      plugin,
      durationMs,
    })),
    routeDistribution,
    averagePriority:
      documents.length > 0 ? Number((totalPriority / documents.length).toFixed(2)) : 0,
    digestSample: documents.slice(0, 6).map((document) => document.digest),
    payloadBytes: 0,
  };
  result.payloadBytes = JSON.stringify(result).length;
  return result;
}
