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

export async function fetch(request: HttpFetchRequest): Promise<HttpFetchResponse> {
  const controller = new AbortController();
  const timeoutMs = request.timeoutMs;
  const timeout = typeof timeoutMs === 'number' && Number.isFinite(timeoutMs) && timeoutMs > 0
    ? setTimeout(() => controller.abort(), timeoutMs)
    : null;

  try {
    const response = await globalThis.fetch(request.url, {
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
