interface VoltBridge {
  invoke<T = unknown>(method: string, args?: unknown): Promise<T>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

interface AnalyticsProfile {
  datasetSize: number;
  cachedSizes: number[];
  categorySpread: Record<string, number>;
  regionSpread: Record<string, number>;
}

interface AnalyticsBenchmarkResult {
  datasetSize: number;
  iterations: number;
  backendDurationMs: number;
  peakMatches: number;
  payloadBytes: number;
}

const bridge = window.__volt__;
if (!bridge?.invoke) {
  throw new Error('window.__volt__.invoke is unavailable');
}

const datasetSizeInput = getInput('dataset-size');
const iterationsInput = getInput('iterations');
const searchTermInput = getInput('search-term');
const minScoreInput = getInput('min-score');
const topNInput = getInput('top-n');
const statusLine = getElement('status-line');
const profileOutput = getElement('profile-output');
const resultOutput = getElement('result-output');
const metricRows = getElement('metric-rows');
const metricBackend = getElement('metric-backend');
const metricRoundTrip = getElement('metric-roundtrip');
const metricMatches = getElement('metric-matches');
const runButton = document.querySelector<HTMLButtonElement>('#run-benchmark');

runButton?.addEventListener('click', () => {
  void runBenchmark();
});

void refreshProfile();

async function refreshProfile(): Promise<void> {
  statusLine.textContent = 'Building dataset profile...';
  const datasetSize = Number(datasetSizeInput.value);
  const profile = await bridge.invoke<AnalyticsProfile>('analytics:profile', { datasetSize });
  metricRows.textContent = String(profile.datasetSize);
  profileOutput.textContent = JSON.stringify(profile, null, 2);
  statusLine.textContent = 'Profile loaded. Ready to benchmark.';
}

async function runBenchmark(): Promise<void> {
  runButton?.setAttribute('disabled', 'disabled');
  statusLine.textContent = 'Running backend filter/sort/group loop...';

  try {
    const startedAt = performance.now();
    const result = await bridge.invoke<AnalyticsBenchmarkResult>('analytics:run', {
      datasetSize: Number(datasetSizeInput.value),
      iterations: Number(iterationsInput.value),
      searchTerm: searchTermInput.value,
      minScore: Number(minScoreInput.value),
      topN: Number(topNInput.value),
    });
    const roundTripMs = Math.round(performance.now() - startedAt);

    metricRows.textContent = String(result.datasetSize);
    metricBackend.textContent = `${result.backendDurationMs} ms`;
    metricRoundTrip.textContent = `${roundTripMs} ms`;
    metricMatches.textContent = String(result.peakMatches);
    resultOutput.textContent = JSON.stringify({
      ...result,
      roundTripMs,
    }, null, 2);
    statusLine.textContent = `Finished. Backend ${result.backendDurationMs} ms, round trip ${roundTripMs} ms, payload ${result.payloadBytes} bytes.`;
  } catch (error) {
    statusLine.textContent = error instanceof Error ? error.message : String(error);
  } finally {
    runButton?.removeAttribute('disabled');
  }
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
