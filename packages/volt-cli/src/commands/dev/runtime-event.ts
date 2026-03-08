export function extractNativeEventJson(callbackArgs: unknown[]): string | null {
  if (callbackArgs.length === 0) {
    return null;
  }

  const [first, second] = callbackArgs;
  if (typeof first === 'string') {
    return first;
  }
  if ((first === null || typeof first === 'undefined') && typeof second === 'string') {
    // N-API threadsafe callbacks commonly arrive as (err, value).
    return second;
  }
  return null;
}
