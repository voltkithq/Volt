# Contributing to Volt

Thanks for contributing.
This project combines Rust runtime code and TypeScript framework/CLI code, so most changes touch more than one layer.
Please follow `CODE_OF_CONDUCT.md` in all project interactions.

## Prerequisites

- Node.js `>=20`
- pnpm `>=9`
- Rust stable toolchain
- Linux only system deps:
  - `libwebkit2gtk-4.1-dev`
  - `libgtk-3-dev`
  - `libayatana-appindicator3-dev`
  - `librsvg2-dev`

## Setup

```bash
pnpm install
pnpm build
```

## Development Workflow

1. Create a branch from `main`.
2. Implement changes with tests in the same PR.
3. Run full validation locally before opening/updating the PR.
4. Keep behavior/docs/contracts in sync.

## Required Validation Before PR

```bash
pnpm typecheck
pnpm test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Contracts and Compatibility

If your change affects public API shape, native event payload contracts, or runtime behavior:

1. Update the relevant contract/docs in the same PR.
2. Run:
   - `pnpm test:contract-compat`
3. Include migration notes for breaking behavior.

## Code Expectations

- Prefer root-cause fixes over local patches.
- Keep platform-specific behavior explicit (Windows/macOS/Linux).
- Add regression tests for each bug fix.
- Keep commits and PR descriptions auditable.

## Pull Request Checklist

- [ ] Change is covered by tests
- [ ] Full validation commands pass
- [ ] Docs/contracts updated (if behavior changed)
- [ ] No new clippy/typecheck/lint regressions

## License

By contributing, you agree your contributions are licensed under the Business Source License 1.1 (BSL 1.1) as described in the `LICENSE` file.
