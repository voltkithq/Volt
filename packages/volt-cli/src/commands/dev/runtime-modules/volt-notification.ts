import { Notification } from 'voltkit';

interface NotificationOptions {
  title: string;
  body?: string;
  icon?: string;
}

export function show(options: NotificationOptions): void {
  new Notification(options).show();
}

