import { invoke, typedInvoke } from './bridge.js';
import type { DomRefs } from './dom.js';
import { renderError, renderInvokeError, renderJson } from './render.js';
import type {
  DbRecord,
  SecureStorageGetResponse,
  SecureStorageHasResponse,
  SecureStorageSetResponse,
  StatusResponse,
  WindowAction,
} from './types.js';
import type {
  ComputeArgs,
  ComputeResponse,
  EchoPayload,
  PingResult,
} from '../ipc-contract.js';

export interface DemoHandlers {
  handlePing(): Promise<void>;
  handleEcho(): Promise<void>;
  handleCompute(): Promise<void>;
  handleStatus(): Promise<void>;
  handleNativeSetup(): Promise<void>;
  handleWindowAction(action: WindowAction): Promise<void>;
  handleProgress(): Promise<void>;
  handleDbAdd(): Promise<void>;
  handleDbList(): Promise<void>;
  handleDbClear(): Promise<void>;
  handleSecretSet(): Promise<void>;
  handleSecretGet(): Promise<void>;
  handleSecretHas(): Promise<void>;
  handleSecretDelete(): Promise<void>;
}

export function createDemoHandlers(dom: DomRefs): DemoHandlers {
  function resolveSecretKey(): string {
    return dom.secretKeyInput.value.trim();
  }

  async function handlePing(): Promise<void> {
    const startedAt = performance.now();
    try {
      const response: PingResult = await typedInvoke.invoke('demo.ping', null);
      const latencyMs = Math.round((performance.now() - startedAt) * 100) / 100;
      dom.latencyValue.textContent = `${latencyMs} ms`;
      renderJson(dom.pingResult, response);
    } catch (error) {
      renderInvokeError(dom.pingResult, error);
    }
  }

  async function handleEcho(): Promise<void> {
    try {
      const payload: EchoPayload = {
        message: dom.echoInput.value,
        sentAt: new Date().toISOString(),
      };
      const response = await typedInvoke.invoke('demo.echo', payload);
      renderJson(dom.echoResult, response);
    } catch (error) {
      renderInvokeError(dom.echoResult, error);
    }
  }

  async function handleCompute(): Promise<void> {
    const a = Number(dom.computeAInput.value);
    const b = Number(dom.computeBInput.value);
    if (!Number.isFinite(a) || !Number.isFinite(b)) {
      dom.computeResult.textContent = 'Error: Both compute values must be valid numbers.';
      return;
    }

    try {
      const payload: ComputeArgs = { a, b };
      const response: ComputeResponse = await typedInvoke.invoke('demo.compute', payload);
      renderJson(dom.computeResult, response);
    } catch (error) {
      renderInvokeError(dom.computeResult, error);
    }
  }

  async function handleStatus(): Promise<void> {
    try {
      const response = await invoke<StatusResponse>('status');
      dom.osValue.textContent = `${response.os.platform} / ${response.os.arch}`;
      dom.uuidValue.textContent = response.generatedUuid;
      dom.clipboardValue.textContent = response.clipboard.hasText ? 'has text' : 'empty';
      dom.windowsValue.textContent = String(response.runtime.windowCount);
      dom.dbRowsValue.textContent = String(response.runtime.dbRows);
      dom.shortcutValue.textContent = response.runtime.shortcut;
      dom.nativeValue.textContent = response.runtime.nativeReady ? 'ready' : 'not setup';
      dom.secureStorageValue.textContent = response.runtime.secureStorageHasDemoKey
        ? `stored (${response.runtime.secureStorageDemoKey})`
        : `empty (${response.runtime.secureStorageDemoKey})`;
      renderJson(dom.statusResult, response);
    } catch (error) {
      renderError(dom.statusResult, error);
    }
  }

  async function handleNativeSetup(): Promise<void> {
    try {
      const response = await invoke<unknown>('native:setup');
      renderJson(dom.nativeResult, response);
      await handleStatus();
    } catch (error) {
      renderError(dom.nativeResult, error);
    }
  }

  async function handleWindowAction(action: WindowAction): Promise<void> {
    try {
      const response = await invoke<unknown>(action);
      renderJson(dom.windowResult, response);
    } catch (error) {
      renderError(dom.windowResult, error);
    }
  }

  async function handleProgress(): Promise<void> {
    dom.progressResult.textContent = 'Progress: starting...';
    try {
      const response = await invoke<unknown>('progress:run');
      renderJson(dom.progressResult, response);
    } catch (error) {
      renderError(dom.progressResult, error);
    }
  }

  async function handleDbAdd(): Promise<void> {
    const message = dom.dbInput.value.trim();
    if (!message) {
      dom.dbResult.textContent = 'Error: Enter a message first.';
      return;
    }

    try {
      const response = await invoke<DbRecord>('db:add', { message });
      dom.dbInput.value = '';
      renderJson(dom.dbResult, response);
      await handleDbList();
      await handleStatus();
    } catch (error) {
      renderError(dom.dbResult, error);
    }
  }

  async function handleDbList(): Promise<void> {
    try {
      const rows = await invoke<DbRecord[]>('db:list');
      renderJson(dom.dbResult, rows);
    } catch (error) {
      renderError(dom.dbResult, error);
    }
  }

  async function handleDbClear(): Promise<void> {
    try {
      const response = await invoke<unknown>('db:clear');
      renderJson(dom.dbResult, response);
      await handleStatus();
    } catch (error) {
      renderError(dom.dbResult, error);
    }
  }

  async function handleSecretSet(): Promise<void> {
    const key = resolveSecretKey();
    const value = dom.secretValueInput.value;
    if (!key) {
      dom.secretResult.textContent = 'Error: Enter a secure storage key.';
      return;
    }
    if (!value.trim()) {
      dom.secretResult.textContent = 'Error: Enter a non-empty secret value.';
      return;
    }

    try {
      const response = await invoke<SecureStorageSetResponse>('secure-storage:set', { key, value });
      renderJson(dom.secretResult, response);
      await handleStatus();
    } catch (error) {
      renderError(dom.secretResult, error);
    }
  }

  async function handleSecretGet(): Promise<void> {
    const key = resolveSecretKey();
    if (!key) {
      dom.secretResult.textContent = 'Error: Enter a secure storage key.';
      return;
    }

    try {
      const response = await invoke<SecureStorageGetResponse>('secure-storage:get', { key });
      renderJson(dom.secretResult, response);
    } catch (error) {
      renderError(dom.secretResult, error);
    }
  }

  async function handleSecretHas(): Promise<void> {
    const key = resolveSecretKey();
    if (!key) {
      dom.secretResult.textContent = 'Error: Enter a secure storage key.';
      return;
    }

    try {
      const response = await invoke<SecureStorageHasResponse>('secure-storage:has', { key });
      renderJson(dom.secretResult, response);
    } catch (error) {
      renderError(dom.secretResult, error);
    }
  }

  async function handleSecretDelete(): Promise<void> {
    const key = resolveSecretKey();
    if (!key) {
      dom.secretResult.textContent = 'Error: Enter a secure storage key.';
      return;
    }

    try {
      const response = await invoke<SecureStorageSetResponse>('secure-storage:delete', { key });
      renderJson(dom.secretResult, response);
      await handleStatus();
    } catch (error) {
      renderError(dom.secretResult, error);
    }
  }

  return {
    handlePing,
    handleEcho,
    handleCompute,
    handleStatus,
    handleNativeSetup,
    handleWindowAction,
    handleProgress,
    handleDbAdd,
    handleDbList,
    handleDbClear,
    handleSecretSet,
    handleSecretGet,
    handleSecretHas,
    handleSecretDelete,
  };
}
