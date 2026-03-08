export interface AutomationEvent {
  event: string;
  payload: unknown;
}

export interface MenuSetupState {
  shortcut: string;
  shortcutRegistered: boolean;
}

export interface MenuAutomationDriverOptions {
  clickEventName?: string;
}

export class MenuAutomationDriver {
  private readonly clickEventName: string;

  public constructor(options: MenuAutomationDriverOptions = {}) {
    this.clickEventName = options.clickEventName ?? 'demo:menu-click';
  }

  public parseSetupPayload(payload: unknown): MenuSetupState {
    const value = asRecord(payload);
    if (!value) {
      throw new Error('[volt:test] invalid menu setup payload: expected object.');
    }

    const shortcut = value.shortcut;
    if (typeof shortcut !== 'string' || shortcut.trim().length === 0) {
      throw new Error('[volt:test] invalid menu setup payload: missing shortcut string.');
    }

    const shortcutRegistered = value.shortcutRegistered;
    if (typeof shortcutRegistered !== 'boolean') {
      throw new Error('[volt:test] invalid menu setup payload: missing shortcutRegistered boolean.');
    }

    return {
      shortcut,
      shortcutRegistered,
    };
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
