export interface WorkflowDocument {
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

export interface WorkflowPluginDefinition {
  name: string;
  label: string;
  description: string;
  run(documents: WorkflowDocument[], pass: number): void;
}

const cleanupRegex = /[^a-z0-9\s]+/g;
const whitespaceRegex = /\s+/g;

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

export const workflowPlugins: WorkflowPluginDefinition[] = [
  {
    name: 'normalizeText',
    label: 'Normalize Text',
    description: 'Lowercases, strips noise, and builds the canonical payload string.',
    run: normalizeText,
  },
  {
    name: 'extractSignals',
    label: 'Extract Signals',
    description: 'Tokenizes content and expands tags from the signal dictionary.',
    run: extractSignals,
  },
  {
    name: 'scorePriority',
    label: 'Score Priority',
    description: 'Assigns risk and priority weights from tokens, tags, and region.',
    run: scorePriority,
  },
  {
    name: 'routeQueues',
    label: 'Route Queues',
    description: 'Turns the scored document into a queue placement decision.',
    run: routeQueues,
  },
  {
    name: 'buildDigests',
    label: 'Build Digests',
    description: 'Produces the final summary string shipped back to the UI.',
    run: buildDigests,
  },
];

export function listWorkflowPlugins(): Array<Pick<WorkflowPluginDefinition, 'name' | 'label' | 'description'>> {
  return workflowPlugins.map(({ name, label, description }) => ({
    name,
    label,
    description,
  }));
}
