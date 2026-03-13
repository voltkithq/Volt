import { fs as frameworkFs } from 'voltkit';
import type { FileInfo } from 'voltkit';

export async function readFile(path: string): Promise<string> {
  return frameworkFs.readFile(path);
}

export async function writeFile(path: string, data: string): Promise<void> {
  await frameworkFs.writeFile(path, data);
}

export async function readDir(path: string): Promise<string[]> {
  return frameworkFs.readDir(path);
}

export async function stat(path: string): Promise<FileInfo> {
  return frameworkFs.stat(path);
}

export async function exists(path: string): Promise<boolean> {
  return frameworkFs.exists(path);
}

export async function mkdir(path: string): Promise<void> {
  await frameworkFs.mkdir(path);
}

export async function remove(path: string): Promise<void> {
  await frameworkFs.remove(path);
}

