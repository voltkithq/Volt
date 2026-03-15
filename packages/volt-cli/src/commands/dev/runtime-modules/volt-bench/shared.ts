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
  weight: number;
  run(documents: WorkflowDocument[], pass: number): void;
}

const cleanupRegex = /[^a-z0-9\s]+/g;
const whitespaceRegex = /\s+/g;

export function toPositiveInteger(
  value: unknown,
  fallback: number,
  min: number,
  max: number,
): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, Math.round(numeric)));
}

export function toPseudoDuration(
  units: number,
  divisor: number,
  baseline = 1,
  modifier = 0,
): number {
  return Math.max(1, Math.round(units / divisor) + baseline + modifier);
}

export function normalizeWorkflowText(value: string): string {
  return value.replace(cleanupRegex, ' ').replace(whitespaceRegex, ' ').trim();
}
