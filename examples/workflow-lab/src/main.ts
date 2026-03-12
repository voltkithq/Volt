import {
  collectSelectedPipeline,
  formatWorkflowStatus,
  loadWorkflowPlugins,
  runWorkflowPipeline,
  type WorkflowFormState,
} from './workflow-client.js';

const batchSizeInput = getInput('batch-size');
const passesInput = getInput('passes');
const statusLine = getElement('status-line');
const pluginList = getElement('plugin-list');
const pipelineOutput = getElement('pipeline-output');
const resultOutput = getElement('result-output');
const metricPlugins = getElement('metric-plugins');
const metricDocs = getElement('metric-docs');
const metricBackend = getElement('metric-backend');
const metricPayload = getElement('metric-payload');
const runButton = document.querySelector<HTMLButtonElement>('#run-workflow-benchmark');

runButton?.addEventListener('click', () => {
  void runBenchmark();
});

void loadPlugins();

async function loadPlugins(): Promise<void> {
  const plugins = await loadWorkflowPlugins();
  pluginList.innerHTML = '';
  for (const plugin of plugins) {
    const wrapper = document.createElement('label');
    wrapper.className = 'plugin-item';
    wrapper.innerHTML = `
      <input type="checkbox" data-plugin-name="${plugin.name}" checked>
      <span>
        <strong>${plugin.label}</strong>
        <span>${plugin.description}</span>
      </span>
    `;
    pluginList.appendChild(wrapper);
  }
  metricPlugins.textContent = String(plugins.length);
  statusLine.textContent = 'Plugin registry loaded. Choose a pipeline and run it.';
}

async function runBenchmark(): Promise<void> {
  const pipeline = collectSelectedPipeline(Array.from(
    pluginList.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'),
  ).map((input) => ({
    checked: input.checked,
    pluginName: input.dataset.pluginName,
  })));

  if (pipeline.length === 0) {
    statusLine.textContent = 'Select at least one plugin before running the workflow benchmark.';
    return;
  }

  runButton?.setAttribute('disabled', 'disabled');
  statusLine.textContent = 'Running plugin pipeline in the main process...';

  try {
    const result = await runWorkflowPipeline(readWorkflowForm(pipeline));

    metricPlugins.textContent = String(result.pipeline.length);
    metricDocs.textContent = `${result.batchSize} x ${result.passes}`;
    metricBackend.textContent = `${result.backendDurationMs} ms`;
    metricPayload.textContent = `${result.payloadBytes} B`;
    pipelineOutput.textContent = JSON.stringify({
      pipeline: result.pipeline,
      backendDurationMs: result.backendDurationMs,
      payloadBytes: result.payloadBytes,
    }, null, 2);
    resultOutput.textContent = JSON.stringify(result, null, 2);
    statusLine.textContent = formatWorkflowStatus(result);
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

function readWorkflowForm(pipeline: string[]): WorkflowFormState {
  return {
    batchSize: Number(batchSizeInput.value),
    passes: Number(passesInput.value),
    pipeline,
  };
}
