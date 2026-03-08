# Notification

OS-level notifications. Requires `permissions: ['notification']`.

## `Notification`

### Constructor

```ts
import { Notification } from 'voltkit';

const notification = new Notification({
  title: 'New Message',
  body: 'You have 3 unread messages',
  icon: './assets/icon.png',
});
```

**Options:** `NotificationOptions`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | `string` | Yes | Notification title |
| `body` | `string` | No | Body text |
| `icon` | `string` | No | Path to an icon file |

### `notification.show(): void`

Display the notification.

```ts
notification.show();
```

### `Notification.isSupported(): boolean`

Check if the OS supports notifications. Always returns `true` on supported platforms.

```ts
if (Notification.isSupported()) {
  new Notification({ title: 'Hello' }).show();
}
```

### Properties

All properties are `readonly`:

| Property | Type | Description |
|----------|------|-------------|
| `title` | `string` | The notification title |
| `body` | `string \| undefined` | The body text |
| `icon` | `string \| undefined` | Path to the icon |
