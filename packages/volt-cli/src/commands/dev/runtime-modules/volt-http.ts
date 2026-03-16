import { lookup } from 'node:dns/promises';
import { isIP } from 'node:net';
import { ensureDevPermission } from './shared.js';

interface HttpFetchRequest {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: unknown;
  timeoutMs?: number;
}

interface HttpFetchResponse {
  status: number;
  headers: Record<string, string[]>;
  text(): Promise<string>;
  json(): Promise<unknown>;
}

const BLOCKED_HOSTNAMES = new Set([
  'localhost',
  '127.0.0.1',
  '0.0.0.0',
  '::1',
  '169.254.169.254',
  'metadata.google.internal',
]);

function normalizeBody(body: unknown): unknown {
  if (body === null || body === undefined) {
    return undefined;
  }
  if (
    typeof body === 'string'
    || body instanceof ArrayBuffer
    || body instanceof Blob
    || body instanceof URLSearchParams
    || body instanceof FormData
    || ArrayBuffer.isView(body)
  ) {
    return body;
  }
  return JSON.stringify(body);
}

function normalizeHeaders(headers: Headers): Record<string, string[]> {
  const normalized: Record<string, string[]> = {};
  headers.forEach((value, key) => {
    const existing = normalized[key];
    if (existing) {
      existing.push(value);
      return;
    }
    normalized[key] = [value];
  });
  return normalized;
}

function createBlockedHttpError(reason: string): Error {
  return new Error(`HTTP request blocked in dev mode: ${reason}.`);
}

function normalizeRequestUrl(url: string): URL {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`Invalid HTTP URL: ${url}`);
  }

  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw createBlockedHttpError(`unsupported protocol '${parsed.protocol}'`);
  }
  if (parsed.username || parsed.password) {
    throw createBlockedHttpError('embedded credentials are not allowed');
  }
  return parsed;
}

function isBlockedHostname(hostname: string): boolean {
  const normalized = hostname.trim().toLowerCase();
  return BLOCKED_HOSTNAMES.has(normalized) || normalized.endsWith('.localhost');
}

function isPrivateIpAddress(address: string): boolean {
  const family = isIP(address);
  if (family === 4) {
    const octets = address.split('.').map((part) => Number.parseInt(part, 10));
    if (octets.length !== 4 || octets.some((part) => Number.isNaN(part))) {
      return true;
    }
    const [a, b] = octets;
    return a === 0
      || a === 10
      || a === 127
      || (a === 169 && b === 254)
      || (a === 172 && b >= 16 && b <= 31)
      || (a === 192 && b === 168);
  }

  if (family === 6) {
    const normalized = address.toLowerCase();
    return normalized === '::1'
      || normalized.startsWith('fe8')
      || normalized.startsWith('fe9')
      || normalized.startsWith('fea')
      || normalized.startsWith('feb')
      || normalized.startsWith('fc')
      || normalized.startsWith('fd')
      || normalized.startsWith('::ffff:127.');
  }

  return true;
}

async function ensureSafeRequestTarget(url: URL): Promise<void> {
  if (isBlockedHostname(url.hostname)) {
    throw createBlockedHttpError(`host '${url.hostname}' is not allowed`);
  }

  const addresses = isIP(url.hostname)
    ? [url.hostname]
    : (await lookup(url.hostname, { all: true, verbatim: true })).map((entry) => entry.address);

  if (addresses.length === 0) {
    throw createBlockedHttpError(`host '${url.hostname}' did not resolve to an address`);
  }
  if (addresses.some((address) => isPrivateIpAddress(address))) {
    throw createBlockedHttpError(`host '${url.hostname}' resolved to a private or local address`);
  }
}

export async function fetch(request: HttpFetchRequest): Promise<HttpFetchResponse> {
  ensureDevPermission('http', 'http.fetch()');
  const targetUrl = normalizeRequestUrl(request.url);
  await ensureSafeRequestTarget(targetUrl);

  const controller = new AbortController();
  const timeoutMs = request.timeoutMs;
  const timeout = typeof timeoutMs === 'number' && Number.isFinite(timeoutMs) && timeoutMs > 0
    ? setTimeout(() => controller.abort(), timeoutMs)
    : null;

  try {
    const response = await globalThis.fetch(targetUrl, {
      method: request.method ?? 'GET',
      headers: request.headers,
      body: normalizeBody(request.body) as RequestInit['body'],
      signal: controller.signal,
    });

    const headers = normalizeHeaders(response.headers);
    return {
      status: response.status,
      headers,
      text: async () => response.text(),
      json: async () => response.json(),
    };
  } finally {
    if (timeout) {
      clearTimeout(timeout);
    }
  }
}
