interface IndexRange {
  start: number;
  end: number;
}

interface PropertyValueRange extends IndexRange {
  key: string;
}

function isIdentifierStart(character: string): boolean { return /^[A-Za-z_$]$/u.test(character); }
function isIdentifierPart(character: string): boolean { return /^[A-Za-z0-9_$]$/u.test(character); }
function isWhitespace(character: string): boolean {
  return character === ' ' || character === '\t' || character === '\n' || character === '\r';
}

function skipQuotedString(source: string, startIndex: number): number {
  const quote = source[startIndex];
  let index = startIndex + 1;
  while (index < source.length) {
    if (source[index] === '\\') {
      index += 2;
      continue;
    }
    if (source[index] === quote) {
      return index + 1;
    }
    index += 1;
  }
  return source.length;
}

function skipLineComment(source: string, startIndex: number): number {
  let index = startIndex + 2;
  while (index < source.length && source[index] !== '\n') {
    index += 1;
  }
  return index;
}

function skipBlockComment(source: string, startIndex: number): number {
  let index = startIndex + 2;
  while (index + 1 < source.length && !(source[index] === '*' && source[index + 1] === '/')) {
    index += 1;
  }
  return Math.min(index + 2, source.length);
}

function skipTokenIfStringOrComment(source: string, startIndex: number): number | null {
  const current = source[startIndex];
  const next = source[startIndex + 1];
  if (current === '\'' || current === '"' || current === '`') {
    return skipQuotedString(source, startIndex);
  }
  if (current === '/' && next === '/') {
    return skipLineComment(source, startIndex);
  }
  if (current === '/' && next === '*') {
    return skipBlockComment(source, startIndex);
  }
  return null;
}

function skipWhitespaceAndComments(source: string, startIndex: number): number {
  let index = startIndex;
  while (index < source.length) {
    if (isWhitespace(source[index])) {
      index += 1;
      continue;
    }
    const nextIndex = skipTokenIfStringOrComment(source, index);
    if (nextIndex === null || nextIndex <= index) {
      return index;
    }
    if (source[index] === '\'' || source[index] === '"' || source[index] === '`') {
      return index;
    }
    index = nextIndex;
  }
  return index;
}

function findMatchingBrace(source: string, openBraceIndex: number): number {
  let depth = 0;
  let index = openBraceIndex;
  while (index < source.length) {
    const nextIndex = skipTokenIfStringOrComment(source, index);
    if (nextIndex !== null) {
      index = nextIndex;
      continue;
    }
    if (source[index] === '{') {
      depth += 1;
    } else if (source[index] === '}') {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
    index += 1;
  }
  return -1;
}

function findDefineConfigObjectBounds(source: string): IndexRange | null {
  const needle = 'defineConfig';
  let searchStart = 0;
  while (searchStart < source.length) {
    const callIndex = source.indexOf(needle, searchStart);
    if (callIndex === -1) {
      return null;
    }
    let index = skipWhitespaceAndComments(source, callIndex + needle.length);
    if (source[index] !== '(') {
      searchStart = callIndex + needle.length;
      continue;
    }
    index = skipWhitespaceAndComments(source, index + 1);
    if (source[index] !== '{') {
      searchStart = callIndex + needle.length;
      continue;
    }
    const closeIndex = findMatchingBrace(source, index);
    return closeIndex === -1 ? null : { start: index, end: closeIndex + 1 };
  }
  return null;
}

function findPropertyValueEnd(source: string, valueStart: number): number {
  let index = valueStart;
  let curlyDepth = 0;
  let squareDepth = 0;
  let parenDepth = 0;
  while (index < source.length) {
    const nextIndex = skipTokenIfStringOrComment(source, index);
    if (nextIndex !== null) {
      index = nextIndex;
      continue;
    }
    const current = source[index];
    if (current === '{') {
      curlyDepth += 1;
    } else if (current === '}') {
      if (curlyDepth === 0 && squareDepth === 0 && parenDepth === 0) {
        return index;
      }
      curlyDepth = Math.max(0, curlyDepth - 1);
    } else if (current === '[') {
      squareDepth += 1;
    } else if (current === ']') {
      squareDepth = Math.max(0, squareDepth - 1);
    } else if (current === '(') {
      parenDepth += 1;
    } else if (current === ')') {
      parenDepth = Math.max(0, parenDepth - 1);
    } else if (current === ',' && curlyDepth === 0 && squareDepth === 0 && parenDepth === 0) {
      return index;
    }
    index += 1;
  }
  return source.length;
}

function parsePropertyAt(source: string, startIndex: number): PropertyValueRange | null {
  let index = skipWhitespaceAndComments(source, startIndex);
  if (index >= source.length) {
    return null;
  }
  let key: string;
  if (source[index] === '\'' || source[index] === '"') {
    const endQuote = skipQuotedString(source, index);
    if (endQuote <= index + 1 || endQuote > source.length) {
      return null;
    }
    key = source.slice(index + 1, endQuote - 1);
    index = endQuote;
  } else if (isIdentifierStart(source[index])) {
    const keyStart = index;
    index += 1;
    while (index < source.length && isIdentifierPart(source[index])) {
      index += 1;
    }
    key = source.slice(keyStart, index);
  } else {
    return null;
  }
  index = skipWhitespaceAndComments(source, index);
  if (source[index] !== ':') {
    return null;
  }
  const valueStart = index + 1;
  return {
    key,
    start: valueStart,
    end: findPropertyValueEnd(source, valueStart),
  };
}

function findTopLevelPropertyValueRange(
  objectLiteral: string,
  propertyName: string,
): IndexRange | null {
  let index = 1;
  let depth = 1;
  while (index < objectLiteral.length) {
    const nextIndex = skipTokenIfStringOrComment(objectLiteral, index);
    if (nextIndex !== null) {
      index = nextIndex;
      continue;
    }
    if (objectLiteral[index] === '{') {
      depth += 1;
      index += 1;
      continue;
    }
    if (objectLiteral[index] === '}') {
      depth -= 1;
      index += 1;
      continue;
    }
    if (depth === 1) {
      const property = parsePropertyAt(objectLiteral, index);
      if (property) {
        if (property.key === propertyName) {
          return { start: property.start, end: property.end };
        }
        index = property.end;
        continue;
      }
    }
    index += 1;
  }
  return null;
}

function replacePropertyValue(
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
    objectLiteral.slice(0, propertyRange.start)
    + leadingWhitespace
    + replacementLiteral
    + objectLiteral.slice(propertyRange.end)
  );
}

function replaceWindowTitle(configObject: string, displayNameLiteral: string): string {
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
  const updatedWindowValue = (
    windowValue.slice(0, objectStart)
    + updatedWindowObject
    + windowValue.slice(objectEnd + 1)
  );
  return (
    configObject.slice(0, windowRange.start)
    + updatedWindowValue
    + configObject.slice(windowRange.end)
  );
}

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
    configContent.slice(0, configBounds.start)
    + updatedConfigObject
    + configContent.slice(configBounds.end)
  );
}
