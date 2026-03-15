interface ParsedVersion {
  major: number;
  minor: number;
  patch: number;
}

const VERSION_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\.(0|[1-9]\d*))?(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

export function parseSemverVersion(value: string): ParsedVersion | null {
  const match = value.trim().match(VERSION_RE);
  if (!match) {
    return null;
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3] ?? '0'),
  };
}

export function semverSatisfies(version: string, range: string): boolean {
  const parsed = parseSemverVersion(version);
  if (!parsed) {
    return false;
  }

  return range
    .split(/\s*\|\|\s*/)
    .some((group) => group.split(/\s*&&\s*|\s+/).filter(Boolean).every((token) => testComparator(parsed, token)));
}

function testComparator(version: ParsedVersion, token: string): boolean {
  if (token.startsWith('^')) {
    return testCaret(version, token.slice(1));
  }
  if (token.startsWith('~')) {
    return testTilde(version, token.slice(1));
  }

  const match = token.match(/^(>=|<=|>|<|=)?(.+)$/);
  if (!match) {
    return false;
  }
  const comparator = match[1] ?? '=';
  const expected = parseSemverVersion(match[2]);
  if (!expected) {
    return false;
  }
  const comparison = compareSemver(version, expected);
  return (
    (comparator === '=' && comparison === 0) ||
    (comparator === '>' && comparison > 0) ||
    (comparator === '>=' && comparison >= 0) ||
    (comparator === '<' && comparison < 0) ||
    (comparator === '<=' && comparison <= 0)
  );
}

function testCaret(version: ParsedVersion, raw: string): boolean {
  const base = parseSemverVersion(raw);
  if (!base || compareSemver(version, base) < 0) {
    return false;
  }
  const upper =
    base.major > 0
      ? { major: base.major + 1, minor: 0, patch: 0 }
      : base.minor > 0
        ? { major: 0, minor: base.minor + 1, patch: 0 }
        : { major: 0, minor: 0, patch: base.patch + 1 };
  return compareSemver(version, upper) < 0;
}

function testTilde(version: ParsedVersion, raw: string): boolean {
  const base = parseSemverVersion(raw);
  if (!base || compareSemver(version, base) < 0) {
    return false;
  }
  return compareSemver(version, {
    major: base.major,
    minor: base.minor + 1,
    patch: 0,
  }) < 0;
}

export function compareSemver(left: ParsedVersion, right: ParsedVersion): number {
  return (
    left.major - right.major || left.minor - right.minor || left.patch - right.patch
  );
}
