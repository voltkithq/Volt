interface VoltBridge {
  invoke<T = unknown>(method: string, args?: unknown): Promise<T>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

interface PluginInfo {
  name: string;
  label: string;
  description: string;
}

interface WorkflowBenchmarkResult {
  batchSize: number;
  passes: number;
  pipeline: string[];
  backendDurationMs: number;
  payloadBytes: number;
}

const bridge = window.__volt__;
if (!bridge?.invoke) {
  throw new Error('window.__volt__.invoke is unavailable');
}

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
  const plugins = await bridge.invoke<PluginInfo[]>('workflow:plugins');
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
  const pipeline = Array.from(pluginList.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'))
    .filter((input) => input.checked)
    .map((input) => input.dataset.pluginName ?? '')
    .filter((value) => value.length > 0);

  if (pipeline.length === 0) {
    statusLine.textContent = 'Select at least one plugin before running the workflow benchmark.';
    return;
  }

  runButton?.setAttribute('disabled', 'disabled');
  statusLine.textContent = 'Running plugin pipeline in the main process...';

  try {
    const result = await bridge.invoke<WorkflowBenchmarkResult>('workflow:run', {
      batchSize: Number(batchSizeInput.value),
      passes: Number(passesInput.value),
      pipeline,
    });

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
    statusLine.textContent = `Workflow complete in ${result.backendDurationMs} ms across ${result.pipeline.length} plugins.`;
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
