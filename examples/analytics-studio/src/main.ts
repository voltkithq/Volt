import {
  formatAnalyticsStatus,
  loadAnalyticsProfile,
  runAnalyticsQuery,
  type AnalyticsQueryFormState,
} from './analytics-client.js';

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
  const profile = await loadAnalyticsProfile(datasetSize);
  metricRows.textContent = String(profile.datasetSize);
  profileOutput.textContent = JSON.stringify(profile, null, 2);
  statusLine.textContent = 'Profile loaded. Ready to benchmark.';
}

async function runBenchmark(): Promise<void> {
  runButton?.setAttribute('disabled', 'disabled');
  statusLine.textContent = 'Running backend filter/sort/group loop...';

  try {
    const startedAt = performance.now();
    const result = await runAnalyticsQuery(readAnalyticsQueryForm());
    const roundTripMs = Math.round(performance.now() - startedAt);

    metricRows.textContent = String(result.datasetSize);
    metricBackend.textContent = `${result.backendDurationMs} ms`;
    metricRoundTrip.textContent = `${roundTripMs} ms`;
    metricMatches.textContent = String(result.peakMatches);
    resultOutput.textContent = JSON.stringify({
      ...result,
      roundTripMs,
    }, null, 2);
    statusLine.textContent = formatAnalyticsStatus(result, roundTripMs);
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

function readAnalyticsQueryForm(): AnalyticsQueryFormState {
  return {
    datasetSize: Number(datasetSizeInput.value),
    iterations: Number(iterationsInput.value),
    searchTerm: searchTermInput.value,
    minScore: Number(minScoreInput.value),
    topN: Number(topNInput.value),
  };
}
