import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { VoltAppLauncher } from '../launcher.js';
import type { VoltTestSuite } from '../types.js';

const RESULT_FILE = '.volt-benchmark-result.json';

export interface WorkflowLabBenchmarkSuiteOptions {
  name?: string;
  projectDir?: string;
  timeoutMs?: number;
}

export interface WorkflowLabBenchmarkPayload {
  ok: boolean;
  plugins: Array<{ name: string; label: string }>;
  result: {
    batchSize: number;
    passes: number;
    pipeline: string[];
    backendDurationMs: number;
    payloadBytes: number;
  };
}

export function createWorkflowLabBenchmarkSuite(
  options: WorkflowLabBenchmarkSuiteOptions = {},
): VoltTestSuite {
  const name = options.name ?? 'workflow-lab-benchmark';
  const projectDir = options.projectDir ?? 'examples/workflow-lab';
  const timeoutMs = options.timeoutMs ?? 900_000;

  return {
    name,
    timeoutMs,
    async run(context) {
      const launcher = new VoltAppLauncher({
        repoRoot: context.repoRoot,
        cliEntryPath: context.cliEntryPath,
        logger: context.logger,
      });

      await launcher.run<WorkflowLabBenchmarkPayload>({
        sourceProjectDir: projectDir,
        resultFile: RESULT_FILE,
        timeoutMs: context.timeoutMs,
        prepareProject(projectDirPath) {
          writeFileSync(join(projectDirPath, 'src', 'main.ts'), WORKFLOW_LAB_AUTORUN_SOURCE, 'utf8');
        },
        validatePayload: validateWorkflowPayload,
        artifactsDir: context.artifactsDir,
      });
    },
  };
}

function validateWorkflowPayload(payload: unknown): WorkflowLabBenchmarkPayload {
  const value = asRecord(payload);
  if (!value || value.ok !== true) {
    throw new Error(`[volt:test] workflow-lab benchmark failed: ${JSON.stringify(payload)}`);
  }

  const plugins = Array.isArray(value.plugins) ? value.plugins : [];
  const result = asRecord(value.result);
  if (!result) {
    throw new Error('[volt:test] workflow-lab payload missing result object.');
  }

  return {
    ok: true,
    plugins: plugins
      .map((plugin) => asRecord(plugin))
      .filter((plugin): plugin is Record<string, unknown> => plugin !== null)
      .map((plugin) => ({
        name: String(plugin.name),
        label: String(plugin.label),
      })),
    result: {
      batchSize: Number(result.batchSize),
      passes: Number(result.passes),
      pipeline: Array.isArray(result.pipeline)
        ? result.pipeline.filter((entry): entry is string => typeof entry === 'string')
        : [],
      backendDurationMs: Number(result.backendDurationMs),
      payloadBytes: Number(result.payloadBytes),
    },
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

const WORKFLOW_LAB_AUTORUN_SOURCE = `
interface VoltBridge {
  invoke<T = unknown>(method: string, args?: unknown): Promise<T>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

async function run(): Promise<void> {
  const bridge = window.__volt__;
  if (!bridge?.invoke) {
    throw new Error('window.__volt__.invoke is unavailable');
  }

  try {
    const plugins = await bridge.invoke<Array<{ name: string; label: string }>>('workflow:plugins');
    const result = await bridge.invoke('workflow:run', {
      batchSize: 6500,
      passes: 4,
      pipeline: plugins.map((plugin) => plugin.name),
    });
    await bridge.invoke('benchmark:complete', {
      ok: true,
      plugins,
      result,
    });
  } catch (error) {
    await bridge.invoke('benchmark:complete', {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

void run();
`.trimStart();
