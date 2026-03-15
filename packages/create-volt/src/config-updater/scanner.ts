export function isIdentifierStart(character: string): boolean {
  return /^[A-Za-z_$]$/u.test(character);
}

export function isIdentifierPart(character: string): boolean {
  return /^[A-Za-z0-9_$]$/u.test(character);
}

export function isWhitespace(character: string): boolean {
  return character === ' ' || character === '\t' || character === '\n' || character === '\r';
}

export function skipQuotedString(source: string, startIndex: number): number {
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

export function skipLineComment(source: string, startIndex: number): number {
  let index = startIndex + 2;
  while (index < source.length && source[index] !== '\n') {
    index += 1;
  }
  return index;
}

export function skipBlockComment(source: string, startIndex: number): number {
  let index = startIndex + 2;
  while (index + 1 < source.length && !(source[index] === '*' && source[index + 1] === '/')) {
    index += 1;
  }
  return Math.min(index + 2, source.length);
}

export function skipTokenIfStringOrComment(source: string, startIndex: number): number | null {
  const current = source[startIndex];
  const next = source[startIndex + 1];
  if (current === "'" || current === '"' || current === '`') {
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

export function skipWhitespaceAndComments(source: string, startIndex: number): number {
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
    if (source[index] === "'" || source[index] === '"' || source[index] === '`') {
      return index;
    }
    index = nextIndex;
  }
  return index;
}

export function findMatchingBrace(source: string, openBraceIndex: number): number {
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
