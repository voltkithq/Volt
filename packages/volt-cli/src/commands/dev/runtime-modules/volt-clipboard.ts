import { clipboard } from 'voltkit';

export function readText(): string {
  return clipboard.readText();
}

export function writeText(text: string): void {
  clipboard.writeText(text);
}

