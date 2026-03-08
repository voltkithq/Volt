# Compatibility and Deprecation Policy

This policy defines Volt's semver, compatibility, and deprecation rules for framework, native bridge, and runtime event contracts.

## Scope

Compatibility commitments apply to:

- `voltkit` public TypeScript API
- N-API exported functions/classes consumed by framework runtime
- Native event payload contracts forwarded to JavaScript
- Configuration schema in `volt.config.ts`

## Versioning Rules

- Volt uses semantic versioning.
- Current release line is pre-1.0 (`0.x`):
  - breaking changes are only allowed in minor releases
  - patch releases must be backward-compatible
- Post-1.0 target behavior:
  - breaking changes only in major releases
  - additive backward-compatible changes in minor releases
  - bug fixes and internal-only changes in patch releases

## Compatibility Guarantees

- Stable APIs must keep:
  - function/class names
  - argument meaning and order
  - return shape semantics
- Event payload contracts must preserve existing required fields.
- New fields may be added as optional (forward-compatible).
- Error codes are treated as stable contract values once documented.

## Deprecation Lifecycle

For public APIs and event fields:

- Step 1: Mark deprecated in docs and changelog.
- Step 2: Add deprecation annotation/message in TypeScript API (when applicable).
- Step 3: Keep old behavior available for at least one minor release in `0.x`.
- Step 4: Remove only in a planned breaking release with migration notes.

## Runtime Bridge Change Policy

- Bridge/internal refactors are allowed in patch/minor releases only if external behavior remains compatible.
- Any change that alters:
  - event payload schema
  - IPC error code taxonomy
  - native API symbols

must follow breaking-change process and migration documentation.

## Release Documentation Requirements

Every release with user-visible changes must include:

- changelog entries grouped as Added/Changed/Fixed/Deprecated/Removed
- explicit migration notes for breaking/deprecated items
- event/schema compatibility notes if payload shape changes

## CI and Validation Expectations

- Compatibility-sensitive areas should be covered by:
  - payload contract tests
  - IPC structured error tests
  - runtime integration tests for request/response paths
  - platform matrix CI jobs for native behavior surfaces
- Contract fixture updates under `contracts/*.json` must include:
  - a compatibility annotation in `contracts/annotations/`
  - a `CHANGELOG.md` update
  - and, for breaking changes, a `docs/compatibility-policy.md` update
- CI gate script: `scripts/contracts/check-compatibility.mjs`
