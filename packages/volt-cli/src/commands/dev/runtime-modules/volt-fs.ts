import { fs as frameworkFs } from 'voltkit';

export async function readFile(path: string): Promise<string> {
  return frameworkFs.readFile(path);
}

export async function writeFile(path: string, data: string): Promise<void> {
  await frameworkFs.writeFile(path, data);
}

export async function readDir(path: string): Promise<string[]> {
  return frameworkFs.readDir(path);
}

export async function exists(path: string): Promise<boolean> {
  try {
    await frameworkFs.stat(path);
    return true;
  } catch {
    return false;
  }
}

export async function mkdir(path: string): Promise<void> {
  await frameworkFs.mkdir(path);
}

export async function remove(path: string): Promise<void> {
  await frameworkFs.remove(path);
}

