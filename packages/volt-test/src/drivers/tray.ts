import type { AutomationEvent } from './menu.js';

export interface TraySetupState {
  trayReady: boolean;
}

export interface TrayAutomationDriverOptions {
  clickEventName?: string;
}

export class TrayAutomationDriver {
  private readonly clickEventName: string;

  public constructor(options: TrayAutomationDriverOptions = {}) {
    this.clickEventName = options.clickEventName ?? 'demo:tray-click';
  }

  public parseSetupPayload(payload: unknown): TraySetupState {
    const value = asRecord(payload);
    if (!value) {
      throw new Error('[volt:test] invalid tray setup payload: expected object.');
    }

    const trayReady = value.trayReady;
    if (typeof trayReady !== 'boolean') {
      throw new Error('[volt:test] invalid tray setup payload: missing trayReady boolean.');
    }

    return { trayReady };
  }

  public countClickEvents(events: readonly AutomationEvent[]): number {
    return events.filter((entry) => entry.event === this.clickEventName).length;
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}
