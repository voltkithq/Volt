import { fsWatchClose, fsWatchPoll, fsWatchStart } from '@voltkit/volt-native';

import type { FileWatcher, WatchEvent, WatchOptions } from './types.js';

export function createWatcher(basePath: string, path: string, options?: WatchOptions): FileWatcher {
  const recursive = options?.recursive ?? true;
  const debounceMs = options?.debounceMs ?? 200;
  const watcherId = fsWatchStart(basePath, path, recursive, debounceMs);
  const handlers = new Set<(events: WatchEvent[]) => void>();
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  if (options?.onEvent) {
    handlers.add(options.onEvent);
  }

  function startPolling(): void {
    if (pollInterval) {
      return;
    }

    pollInterval = setInterval(
      () => {
        const events = fsWatchPoll(watcherId) as WatchEvent[];
        if (events.length === 0) {
          return;
        }

        for (const handler of handlers) {
          try {
            handler(events);
          } catch {
            /* handler errors should not crash the watcher */
          }
        }
      },
      Math.max(debounceMs, 50),
    );
  }

  function stopPollingIfEmpty(): void {
    if (handlers.size === 0 && pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  if (handlers.size > 0) {
    startPolling();
  }

  return {
    async poll(): Promise<WatchEvent[]> {
      return fsWatchPoll(watcherId) as WatchEvent[];
    },
    on(event: 'change', handler: (events: WatchEvent[]) => void): void {
      if (event === 'change') {
        handlers.add(handler);
        startPolling();
      }
    },
    off(event: 'change', handler: (events: WatchEvent[]) => void): void {
      if (event === 'change') {
        handlers.delete(handler);
        stopPollingIfEmpty();
      }
    },
    async close(): Promise<void> {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }

      handlers.clear();
      fsWatchClose(watcherId);
    },
  };
}
