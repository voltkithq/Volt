import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { Notification } from '../notification.js';
import { notificationShow } from '@voltkit/volt-native';

describe('Notification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('sets title, body, and icon from options', () => {
    const n = new Notification({
      title: 'Test',
      body: 'Body text',
      icon: '/path/icon.png',
    });
    expect(n.title).toBe('Test');
    expect(n.body).toBe('Body text');
    expect(n.icon).toBe('/path/icon.png');
  });

  it('body and icon are optional', () => {
    const n = new Notification({ title: 'Minimal' });
    expect(n.title).toBe('Minimal');
    expect(n.body).toBeUndefined();
    expect(n.icon).toBeUndefined();
  });

  it('show calls native notificationShow', () => {
    const n = new Notification({
      title: 'Alert',
      body: 'Something happened',
    });
    n.show();
    expect(notificationShow).toHaveBeenCalledWith({
      title: 'Alert',
      body: 'Something happened',
      icon: undefined,
    });
  });

  it('isSupported returns true', () => {
    expect(Notification.isSupported()).toBe(true);
  });

  it('rejects empty titles', () => {
    expect(() => new Notification({ title: '   ' })).toThrow(
      'Notification title must be a non-empty string.',
    );
  });
});
