import type { IndexRange, PropertyValueRange } from './types.js';
import {
  findMatchingBrace,
  isIdentifierPart,
  isIdentifierStart,
  skipQuotedString,
  skipTokenIfStringOrComment,
  skipWhitespaceAndComments,
} from './scanner.js';

export function findDefineConfigObjectBounds(source: string): IndexRange | null {
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

export function findPropertyValueEnd(source: string, valueStart: number): number {
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

export function parsePropertyAt(source: string, startIndex: number): PropertyValueRange | null {
  let index = skipWhitespaceAndComments(source, startIndex);
  if (index >= source.length) {
    return null;
  }
  let key: string;
  if (source[index] === "'" || source[index] === '"') {
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

export function findTopLevelPropertyValueRange(
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
