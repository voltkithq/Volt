import { EventEmitter } from 'node:events';
import { createServer } from 'node:http';
import type { ChildProcess } from 'node:child_process';
import { describe, expect, it } from 'vitest';
import { extractViteLocalUrl, resolveViteDevUrl } from '../commands/dev/server.js';

describe('Vite dev server URL detection', () => {
  it('extracts a local URL from standard Vite output', () => {
    const line = '  > Local:   http://localhost:5174/';
    expect(extractViteLocalUrl(line)).toBe('http://localhost:5174');
  });

  it('handles ANSI-colored Vite output', () => {
    const line = '\u001B[32m  > Local:\u001B[39m   http://127.0.0.1:5173/';
    expect(extractViteLocalUrl(line)).toBe('http://127.0.0.1:5173');
  });

  it('ignores non-local URL lines', () => {
    expect(extractViteLocalUrl('  > Network: http://192.168.1.7:5173/')).toBeNull();
    expect(extractViteLocalUrl('Port 5173 is in use, trying another one...')).toBeNull();
  });
});

describe('resolveViteDevUrl', () => {
  it('falls back to the requested URL when Vite local URL is not detected', async () => {
    const server = createServer((_, response) => {
      response.statusCode = 200;
      response.end('ok');
    });

    await new Promise<void>((resolve, reject) => {
      server.listen(0, '127.0.0.1', () => resolve());
      server.once('error', reject);
    });

    try {
      const address = server.address();
      if (!address || typeof address === 'string') {
        throw new Error('Expected TCP server address');
      }

      const fallbackUrl = `http://127.0.0.1:${address.port}`;
      const fakeChild = new EventEmitter() as unknown as ChildProcess;
      const resolved = await resolveViteDevUrl(
        { child: fakeChild, detectedUrl: Promise.resolve(null) },
        fallbackUrl,
        2000,
      );

      expect(resolved).toBe(fallbackUrl);
    } finally {
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  });

  it('prefers a detected local URL when available', async () => {
    const server = createServer((_, response) => {
      response.statusCode = 200;
      response.end('ok');
    });

    await new Promise<void>((resolve, reject) => {
      server.listen(0, '127.0.0.1', () => resolve());
      server.once('error', reject);
    });

    try {
      const address = server.address();
      if (!address || typeof address === 'string') {
        throw new Error('Expected TCP server address');
      }

      const baseUrl = `http://127.0.0.1:${address.port}`;
      const fakeChild = new EventEmitter() as unknown as ChildProcess;
      const resolved = await resolveViteDevUrl(
        { child: fakeChild, detectedUrl: Promise.resolve(`${baseUrl}/`) },
        'http://127.0.0.1:9',
        2000,
      );

      expect(resolved).toBe(baseUrl);
    } finally {
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  });

  it('returns null when neither detected nor fallback URL becomes reachable', async () => {
    const fakeChild = new EventEmitter() as unknown as ChildProcess;
    const resolved = await resolveViteDevUrl(
      { child: fakeChild, detectedUrl: Promise.resolve(null) },
      'http://127.0.0.1:9',
      300,
    );

    expect(resolved).toBeNull();
  });
});
