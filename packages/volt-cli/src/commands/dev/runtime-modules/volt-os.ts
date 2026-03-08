import nodeOs from 'node:os';

export function platform(): string {
  return nodeOs.platform();
}

export function arch(): string {
  return nodeOs.arch();
}

export function homeDir(): string {
  return nodeOs.homedir();
}

export function tempDir(): string {
  return nodeOs.tmpdir();
}

