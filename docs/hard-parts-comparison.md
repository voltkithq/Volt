# Desktop Framework Hard-Parts Comparison

This page compares the areas that usually make or break desktop app delivery.

Scope:
- practical engineering concerns, not marketing features
- what teams need to ship safely in production
- Volt status based on this repository's current implementation

## Hard Parts Checklist

| Hard Part | Why It Matters | Electron (typical) | Tauri (typical) | Volt (current repo) |
|-----------|----------------|--------------------|-----------------|---------------------|
| Permission model | Prevent accidental native API overreach | App-defined conventions | Capability model via config | Capability model via `permissions` in `volt.config.ts` |
| IPC boundary validation | Prevent malformed/untrusted payload issues | Usually custom validation | Command boundary with Serde/Rust typing | Typed IPC contract + runtime validation |
| Updater integrity | Prevent tampered binary updates | Requires explicit signing + verification wiring | Built-in updater patterns | Signed update flow with signature verification and manifest tooling |
| Windows signing/provider support | Enterprise distribution often requires managed signing | Manual provider wiring | Custom integration work | Local + Azure Trusted Signing + DigiCert KeyLocker flows |
| Packaging outputs | Installer formats vary by enterprise policy | NSIS/Squirrel via ecosystem tools | Platform-specific bundlers | NSIS, MSIX, AppImage, deb, macOS app/dmg |
| Enterprise policy rollout | Managed desktops need central policy controls | Usually custom scripts/docs | Usually custom | Generated ADMX/ADML + deployment bundle |
| Native E2E automation | Catch regressions in native shell integrations | Usually external tooling | Usually custom wrappers | Built-in `@voltkit/volt-test` with CI matrix support |
| Dev/runtime parity | Reduce "works in build, fails in dev" risk | Requires discipline | Good defaults | Shared runtime model + IPC contract testing in examples |

## Where Volt Is Strong Today

1. Packaging/signing pipeline is integrated into CLI workflows.
2. Typed IPC contract path enforces runtime validation.
3. Enterprise deployment assets are generated from one packaging pass.
4. E2E foundation exists and runs across target OSes.

## Current Gaps To Keep Improving

1. Docs and onboarding need continuous tightening for first-time users.
2. Cross-platform packaging/signing validation should keep expanding in CI.
3. Enterprise policy surface should evolve with additional managed settings over time.

## Suggested Evaluation Flow

1. Run the 5-minute onboarding in `docs/onboarding-5-minutes.md`.
2. Run `volt doctor` before packaging.
3. Package with the target format(s) and verify produced artifacts.
4. Execute smoke E2E suites for your target OS matrix.
