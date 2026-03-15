import { existsSync } from 'node:fs';

function assertFile(filePath, description) {
  if (!existsSync(filePath)) {
    throw new Error(`[bench] Missing ${description}: ${filePath}`);
  }
}

function parsePrefixedJson(output, prefix) {
  const line = output
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .reverse()
    .find((entry) => entry.startsWith(prefix));

  if (!line) {
    throw new Error(`[bench] Missing ${prefix} marker in command output.`);
  }

  return JSON.parse(line.slice(prefix.length));
}

export { assertFile, parsePrefixedJson };
