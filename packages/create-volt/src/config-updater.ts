import { findDefineConfigObjectBounds } from './config-updater/parsing.js';
import { replaceWindowTitle, replacePropertyValue } from './config-updater/replacement.js';

export function updateVoltConfigContent(configContent: string, displayName: string): string {
  const configBounds = findDefineConfigObjectBounds(configContent);
  if (!configBounds) {
    return configContent;
  }
  const displayNameLiteral = JSON.stringify(displayName);
  const configObject = configContent.slice(configBounds.start, configBounds.end);
  const updatedConfigObject = replaceWindowTitle(
    replacePropertyValue(configObject, 'name', displayNameLiteral),
    displayNameLiteral,
  );
  return (
    configContent.slice(0, configBounds.start) +
    updatedConfigObject +
    configContent.slice(configBounds.end)
  );
}
