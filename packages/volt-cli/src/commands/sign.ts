import { signSetupCommand, __testOnly } from './sign/command.js';

export { signSetupCommand };
export type { SignSetupContext, SignSetupOptions, SignSetupPlatform, SignSetupWindowsProvider } from './sign/types.js';

export const signTestOnly = __testOnly;
