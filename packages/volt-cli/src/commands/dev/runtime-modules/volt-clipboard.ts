import { clipboard } from 'voltkit';
import { ensureDevPermission } from './shared.js';

export function readText(): string {
  ensureDevPermission('clipboard', 'clipboard.readText()');
  return clipboard.readText();
}

export function writeText(text: string): void {
  ensureDevPermission('clipboard', 'clipboard.writeText()');
  clipboard.writeText(text);
}
