#!/usr/bin/env node
// Copy the cargo-built volt-napi cdylib to the location expected by the NAPI-RS loader.
import { copyFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

const napiDir = resolve('crates', 'volt-napi');

const platformMap = {
  linux: { x64: { src: 'libvolt_napi.so', dest: 'volt-native.linux-x64-gnu.node' } },
  win32: { x64: { src: 'volt_napi.dll', dest: 'volt-native.win32-x64-msvc.node' } },
  darwin: {
    x64: { src: 'libvolt_napi.dylib', dest: 'volt-native.darwin-x64.node' },
    arm64: { src: 'libvolt_napi.dylib', dest: 'volt-native.darwin-arm64.node' },
  },
};

const entry = platformMap[process.platform]?.[process.arch];
if (!entry) {
  console.log(`No NAPI binding mapping for ${process.platform}-${process.arch}, skipping.`);
  process.exit(0);
}

for (const profile of ['debug', 'release']) {
  const src = resolve('target', profile, entry.src);
  if (existsSync(src)) {
    const dest = resolve(napiDir, entry.dest);
    copyFileSync(src, dest);
    console.log(`Copied ${src} → ${dest}`);
    process.exit(0);
  }
}

console.warn('No built volt-napi binary found in target/debug or target/release.');
process.exit(1);
