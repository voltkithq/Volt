import { findMatchingBrace, skipWhitespaceAndComments } from './scanner.js';
import { findTopLevelPropertyValueRange } from './parsing.js';

export function replacePropertyValue(
  objectLiteral: string,
  propertyName: string,
  replacementLiteral: string,
): string {
  const propertyRange = findTopLevelPropertyValueRange(objectLiteral, propertyName);
  if (!propertyRange) {
    return objectLiteral;
  }
  const currentValue = objectLiteral.slice(propertyRange.start, propertyRange.end);
  const leadingWhitespace = currentValue.match(/^\s*/u)?.[0] ?? '';
  return (
    objectLiteral.slice(0, propertyRange.start) +
    leadingWhitespace +
    replacementLiteral +
    objectLiteral.slice(propertyRange.end)
  );
}

export function replaceWindowTitle(configObject: string, displayNameLiteral: string): string {
  const windowRange = findTopLevelPropertyValueRange(configObject, 'window');
  if (!windowRange) {
    return configObject;
  }
  const windowValue = configObject.slice(windowRange.start, windowRange.end);
  const objectStart = skipWhitespaceAndComments(windowValue, 0);
  if (windowValue[objectStart] !== '{') {
    return configObject;
  }
  const objectEnd = findMatchingBrace(windowValue, objectStart);
  if (objectEnd === -1) {
    return configObject;
  }
  const windowObject = windowValue.slice(objectStart, objectEnd + 1);
  const updatedWindowObject = replacePropertyValue(windowObject, 'title', displayNameLiteral);
  const updatedWindowValue =
    windowValue.slice(0, objectStart) + updatedWindowObject + windowValue.slice(objectEnd + 1);
  return (
    configObject.slice(0, windowRange.start) +
    updatedWindowValue +
    configObject.slice(windowRange.end)
  );
}
