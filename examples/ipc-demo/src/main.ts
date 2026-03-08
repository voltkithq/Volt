import { getDomRefs } from './main/dom.js';
import { createDemoHandlers } from './main/handlers.js';
import { bindRuntimeEvents, unbindRuntimeEvents } from './main/runtime-events.js';

const dom = getDomRefs();
const handlers = createDemoHandlers(dom);

function bindButton(button: HTMLButtonElement, handler: () => Promise<void>): void {
  button.addEventListener('click', () => {
    void handler();
  });
}

bindButton(dom.pingButton, handlers.handlePing);
bindButton(dom.echoButton, handlers.handleEcho);
bindButton(dom.computeButton, handlers.handleCompute);
bindButton(dom.statusButton, handlers.handleStatus);
bindButton(dom.nativeSetupButton, handlers.handleNativeSetup);
bindButton(dom.progressButton, handlers.handleProgress);
bindButton(dom.dbAddButton, handlers.handleDbAdd);
bindButton(dom.dbListButton, handlers.handleDbList);
bindButton(dom.dbClearButton, handlers.handleDbClear);
bindButton(dom.secretSetButton, handlers.handleSecretSet);
bindButton(dom.secretGetButton, handlers.handleSecretGet);
bindButton(dom.secretHasButton, handlers.handleSecretHas);
bindButton(dom.secretDeleteButton, handlers.handleSecretDelete);

dom.windowMinimizeButton.addEventListener('click', () => {
  void handlers.handleWindowAction('window:minimize');
});
dom.windowMaximizeButton.addEventListener('click', () => {
  void handlers.handleWindowAction('window:maximize');
});
dom.windowRestoreButton.addEventListener('click', () => {
  void handlers.handleWindowAction('window:restore');
});

bindRuntimeEvents(dom, handlers);
if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    unbindRuntimeEvents();
  });
}

void handlers.handleNativeSetup();
void handlers.handleDbList();
void handlers.handleSecretHas();
void handlers.handleStatus();
