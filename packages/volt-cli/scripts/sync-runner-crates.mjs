#!/usr/bin/env node

/**
 * Sync runner crate source into volt-cli for publishing.
 * This allows `volt build` to compile the runner for standalone apps
 * that don't have their own Cargo workspace.
 */

import { cpSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const cliRoot = resolve(__dirname, '..');
const monorepoRoot = resolve(cliRoot, '..', '..');
const outDir = resolve(cliRoot, 'runner-crates');

// Clean and recreate
import { rmSync } from 'node:fs';
rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

// Copy crate source directories (only src/, Cargo.toml, build.rs, assets/)
for (const crate of ['volt-runner', 'volt-core', 'volt-updater-helper']) {
  const src = resolve(monorepoRoot, 'crates', crate);
  const dest = resolve(outDir, 'crates', crate);
  cpSync(src, dest, {
    recursive: true,
    filter: (source) => {
      // Skip target/, node_modules/, .node files, test fixtures
      const rel = source.slice(src.length);
      if (rel.includes('target')) return false;
      if (rel.includes('node_modules')) return false;
      if (rel.endsWith('.node')) return false;
      return true;
    },
  });
}

// Create a minimal workspace Cargo.toml
const workspaceCargo = `[workspace]
members = [
    "crates/volt-core",
    "crates/volt-runner",
    "crates/volt-updater-helper",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "BSL-1.1"

[profile.release]
lto = true
strip = "symbols"
codegen-units = 1
opt-level = "z"
`;
writeFileSync(resolve(outDir, 'Cargo.toml'), workspaceCargo);

// Copy the lockfile for reproducible builds
cpSync(resolve(monorepoRoot, 'Cargo.lock'), resolve(outDir, 'Cargo.lock'));

console.log('[sync-runner-crates] Runner crate source synced to runner-crates/');
