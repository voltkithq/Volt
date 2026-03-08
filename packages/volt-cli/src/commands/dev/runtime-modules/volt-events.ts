import { emitFrontendEvent } from './shared.js';

export function emit(eventName: string, data?: unknown): void {
  emitFrontendEvent(eventName, data);
}

export function emitTo(windowId: string, eventName: string, data?: unknown): void {
  emitFrontendEvent(eventName, data, windowId);
}

