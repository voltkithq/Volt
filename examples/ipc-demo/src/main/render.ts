import { isIpcContractValidationError } from 'voltkit/renderer';

export function renderJson(target: HTMLPreElement, value: unknown): void {
  target.textContent = JSON.stringify(value, null, 2);
}

export function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function renderError(target: HTMLPreElement, error: unknown): void {
  target.textContent = `Error: ${toErrorMessage(error)}`;
}

export function renderInvokeError(target: HTMLPreElement, error: unknown): void {
  if (isIpcContractValidationError(error)) {
    target.textContent = `Contract error (${error.phase} @ ${error.channel}): ${error.message}`;
    return;
  }
  renderError(target, error);
}
