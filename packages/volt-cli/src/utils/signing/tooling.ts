import { execFileSync } from 'node:child_process';

/**
 * Check if a command-line tool is available on the system.
 */
export function isToolAvailable(toolName: string): boolean {
  try {
    const lookupTool = process.platform === 'win32' ? 'where' : 'which';
    execFileSync(lookupTool, [toolName], { stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
}
