interface VoltBridge {
  invoke<T = unknown>(method: string, args?: unknown): Promise<T>;
  on(event: string, callback: (payload: unknown) => void): void;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

interface SyncStatus {
  activeScenarioId: string | null;
  preset: {
    workerCount: number;
    ticksPerWorker: number;
    intervalMs: number;
    burstSize: number;
  };
}

interface SyncSummary {
  totalTickEvents: number;
  snapshotEvents: number;
  averageDriftMs: number;
  maxDriftMs: number;
}

const bridge = window.__volt__;
if (!bridge?.invoke || !bridge.on) {
  throw new Error('window.__volt__ bridge is unavailable');
}

const workerCountInput = getInput('worker-count');
const ticksPerWorkerInput = getInput('ticks-per-worker');
const intervalInput = getInput('interval-ms');
const burstInput = getInput('burst-size');
const statusLine = getElement('status-line');
const eventOutput = getElement('event-output');
const summaryOutput = getElement('summary-output');
const metricEvents = getElement('metric-events');
const metricSnapshots = getElement('metric-snapshots');
const metricLag = getElement('metric-lag');
const metricDrift = getElement('metric-drift');
const runButton = document.querySelector<HTMLButtonElement>('#run-sync-benchmark');

let activeScenarioId: string | null = null;
let eventCount = 0;
let snapshotCount = 0;
let totalLagMs = 0;
const recentEvents: unknown[] = [];

bridge.on('sync:tick', (payload) => {
  const record = payload as { scenarioId?: string; issuedAt?: number; driftMs?: number } | null;
  if (!record || record.scenarioId !== activeScenarioId) {
    return;
  }
  eventCount += 1;
  totalLagMs += Math.max(0, Date.now() - Number(record.issuedAt ?? Date.now()));
  recentEvents.unshift(payload);
  recentEvents.splice(10);
  eventOutput.textContent = JSON.stringify(recentEvents, null, 2);
  metricEvents.textContent = String(eventCount);
  metricLag.textContent = `${eventCount > 0 ? Math.round(totalLagMs / eventCount) : 0} ms`;
  metricDrift.textContent = `${Math.round(Number(record.driftMs ?? 0))} ms`;
});

bridge.on('sync:snapshot', (payload) => {
  const record = payload as { scenarioId?: string } | null;
  if (!record || record.scenarioId !== activeScenarioId) {
    return;
  }
  snapshotCount += 1;
  metricSnapshots.textContent = String(snapshotCount);
});

bridge.on('sync:complete', (payload) => {
  const summary = payload as SyncSummary & { scenarioId?: string };
  if (summary.scenarioId !== activeScenarioId) {
    return;
  }
  runButton?.removeAttribute('disabled');
  activeScenarioId = null;
  metricEvents.textContent = String(summary.totalTickEvents);
  metricSnapshots.textContent = String(summary.snapshotEvents);
  metricLag.textContent = `${eventCount > 0 ? Math.round(totalLagMs / eventCount) : 0} ms`;
  metricDrift.textContent = `${summary.maxDriftMs} ms`;
  summaryOutput.textContent = JSON.stringify({
    ...summary,
    rendererAverageLagMs: eventCount > 0 ? Math.round(totalLagMs / eventCount) : 0,
  }, null, 2);
  statusLine.textContent = `Completed ${summary.totalTickEvents} tick events across ${summary.snapshotEvents} snapshots.`;
});

runButton?.addEventListener('click', () => {
  void runScenario();
});

void loadPreset();

async function loadPreset(): Promise<void> {
  const status = await bridge.invoke<SyncStatus>('sync:status');
  workerCountInput.value = String(status.preset.workerCount);
  ticksPerWorkerInput.value = String(status.preset.ticksPerWorker);
  intervalInput.value = String(status.preset.intervalMs);
  burstInput.value = String(status.preset.burstSize);
  statusLine.textContent = status.activeScenarioId
    ? `Scenario already running: ${status.activeScenarioId}`
    : 'Preset loaded. Ready to launch event storm.';
}

async function runScenario(): Promise<void> {
  runButton?.setAttribute('disabled', 'disabled');
  eventCount = 0;
  snapshotCount = 0;
  totalLagMs = 0;
  recentEvents.length = 0;
  eventOutput.textContent = 'Waiting for backend events...';
  summaryOutput.textContent = 'Scenario active...';
  statusLine.textContent = 'Running worker burst orchestration...';

  const response = await bridge.invoke<{ scenarioId: string }>('sync:run', {
    workerCount: Number(workerCountInput.value),
    ticksPerWorker: Number(ticksPerWorkerInput.value),
    intervalMs: Number(intervalInput.value),
    burstSize: Number(burstInput.value),
  });
  activeScenarioId = response.scenarioId;
  statusLine.textContent = `Scenario ${response.scenarioId} launched. Listening for ticks...`;
}

function getInput(id: string): HTMLInputElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`Missing input: ${id}`);
  }
  return element;
}

function getElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLElement)) {
    throw new Error(`Missing element: ${id}`);
  }
  return element;
}
