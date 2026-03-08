/**
 * Native notification module.
 * Provides OS-level notifications.
 * Requires `permissions: ['notification']` in volt.config.ts.
 */

import { notificationShow } from '@voltkit/volt-native';

/** Options for creating a notification. */
export interface NotificationOptions {
  /** Notification title. */
  title: string;
  /** Notification body text. */
  body?: string;
  /** Path to an icon file. */
  icon?: string;
}

/**
 * Native OS notification.
 * Web Notification API compatible.
 *
 * @example
 * ```ts
 * new Notification({ title: 'New message', body: 'Hello!' }).show();
 * ```
 */
export class Notification {
  readonly title: string;
  readonly body: string | undefined;
  readonly icon: string | undefined;

  constructor(options: NotificationOptions) {
    const title = options.title.trim();
    if (!title) {
      throw new Error('Notification title must be a non-empty string.');
    }
    this.title = title;
    this.body = options.body;
    this.icon = options.icon;
  }

  /** Display the notification. */
  show(): void {
    notificationShow({
      title: this.title,
      body: this.body,
      icon: this.icon,
    });
  }

  /** Check if the OS supports notifications. */
  static isSupported(): boolean {
    return true;
  }
}
