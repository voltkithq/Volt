import { runSuites } from './runner/execution.js';
import { selectSuites, withPrefix, withTimeout } from './runner/helpers.js';

export { runSuites };

export const __testOnly = {
  selectSuites,
  withTimeout,
  withPrefix,
};
