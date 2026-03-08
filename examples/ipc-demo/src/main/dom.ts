export interface DomRefs {
  pingButton: HTMLButtonElement;
  echoButton: HTMLButtonElement;
  computeButton: HTMLButtonElement;
  statusButton: HTMLButtonElement;
  nativeSetupButton: HTMLButtonElement;
  windowMinimizeButton: HTMLButtonElement;
  windowMaximizeButton: HTMLButtonElement;
  windowRestoreButton: HTMLButtonElement;
  progressButton: HTMLButtonElement;
  dbAddButton: HTMLButtonElement;
  dbListButton: HTMLButtonElement;
  dbClearButton: HTMLButtonElement;
  secretSetButton: HTMLButtonElement;
  secretGetButton: HTMLButtonElement;
  secretHasButton: HTMLButtonElement;
  secretDeleteButton: HTMLButtonElement;
  echoInput: HTMLInputElement;
  computeAInput: HTMLInputElement;
  computeBInput: HTMLInputElement;
  dbInput: HTMLInputElement;
  secretKeyInput: HTMLInputElement;
  secretValueInput: HTMLInputElement;
  pingResult: HTMLPreElement;
  echoResult: HTMLPreElement;
  computeResult: HTMLPreElement;
  statusResult: HTMLPreElement;
  nativeResult: HTMLPreElement;
  windowResult: HTMLPreElement;
  progressResult: HTMLPreElement;
  dbResult: HTMLPreElement;
  secretResult: HTMLPreElement;
  eventsResult: HTMLPreElement;
  latencyValue: HTMLSpanElement;
  osValue: HTMLSpanElement;
  uuidValue: HTMLSpanElement;
  clipboardValue: HTMLSpanElement;
  windowsValue: HTMLSpanElement;
  dbRowsValue: HTMLSpanElement;
  shortcutValue: HTMLSpanElement;
  nativeValue: HTMLSpanElement;
  secureStorageValue: HTMLSpanElement;
}

function requireElement<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) {
    throw new Error(`Missing required DOM element: #${id}`);
  }
  return node as T;
}

export function getDomRefs(): DomRefs {
  return {
    pingButton: requireElement<HTMLButtonElement>('btn-ping'),
    echoButton: requireElement<HTMLButtonElement>('btn-echo'),
    computeButton: requireElement<HTMLButtonElement>('btn-compute'),
    statusButton: requireElement<HTMLButtonElement>('btn-status'),
    nativeSetupButton: requireElement<HTMLButtonElement>('btn-native-setup'),
    windowMinimizeButton: requireElement<HTMLButtonElement>('btn-window-minimize'),
    windowMaximizeButton: requireElement<HTMLButtonElement>('btn-window-maximize'),
    windowRestoreButton: requireElement<HTMLButtonElement>('btn-window-restore'),
    progressButton: requireElement<HTMLButtonElement>('btn-progress'),
    dbAddButton: requireElement<HTMLButtonElement>('btn-db-add'),
    dbListButton: requireElement<HTMLButtonElement>('btn-db-list'),
    dbClearButton: requireElement<HTMLButtonElement>('btn-db-clear'),
    secretSetButton: requireElement<HTMLButtonElement>('btn-secret-set'),
    secretGetButton: requireElement<HTMLButtonElement>('btn-secret-get'),
    secretHasButton: requireElement<HTMLButtonElement>('btn-secret-has'),
    secretDeleteButton: requireElement<HTMLButtonElement>('btn-secret-delete'),
    echoInput: requireElement<HTMLInputElement>('echo-input'),
    computeAInput: requireElement<HTMLInputElement>('compute-a'),
    computeBInput: requireElement<HTMLInputElement>('compute-b'),
    dbInput: requireElement<HTMLInputElement>('db-input'),
    secretKeyInput: requireElement<HTMLInputElement>('secret-key'),
    secretValueInput: requireElement<HTMLInputElement>('secret-value'),
    pingResult: requireElement<HTMLPreElement>('result-ping'),
    echoResult: requireElement<HTMLPreElement>('result-echo'),
    computeResult: requireElement<HTMLPreElement>('result-compute'),
    statusResult: requireElement<HTMLPreElement>('result-status'),
    nativeResult: requireElement<HTMLPreElement>('result-native'),
    windowResult: requireElement<HTMLPreElement>('result-window'),
    progressResult: requireElement<HTMLPreElement>('result-progress'),
    dbResult: requireElement<HTMLPreElement>('result-db'),
    secretResult: requireElement<HTMLPreElement>('result-secret'),
    eventsResult: requireElement<HTMLPreElement>('result-events'),
    latencyValue: requireElement<HTMLSpanElement>('status-latency'),
    osValue: requireElement<HTMLSpanElement>('status-os'),
    uuidValue: requireElement<HTMLSpanElement>('status-uuid'),
    clipboardValue: requireElement<HTMLSpanElement>('status-clipboard'),
    windowsValue: requireElement<HTMLSpanElement>('status-windows'),
    dbRowsValue: requireElement<HTMLSpanElement>('status-db-rows'),
    shortcutValue: requireElement<HTMLSpanElement>('status-shortcut'),
    nativeValue: requireElement<HTMLSpanElement>('status-native'),
    secureStorageValue: requireElement<HTMLSpanElement>('status-secure-storage'),
  };
}
