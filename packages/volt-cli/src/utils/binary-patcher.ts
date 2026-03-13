import { readFileSync, writeFileSync } from 'node:fs';

/**
 * Sentinel markers — must match the values in crates/volt-runner/build.rs.
 * These 32-byte sequences are located in the binary to find the placeholder slots.
 */
const SENTINELS = {
  assetBundle: Buffer.from('__VOLT_SENTINEL_ASSET_BUNDLE_V1_', 'ascii'),
  backendBundle: Buffer.from('__VOLT_SENTINEL_BACKEND_BNDL_V1_', 'ascii'),
  runnerConfig: Buffer.from('__VOLT_SENTINEL_RUNNER_CONFG_V1_', 'ascii'),
} as const;

/** Sentinel header (32 bytes) + actual_length field (4 bytes) */
const HEADER_SIZE = 36;

export interface PatchInput {
  assetBundle: Buffer;
  backendBundle: Buffer;
  runnerConfig: Buffer;
}

/**
 * Patch a pre-built runner shell binary with real app data.
 *
 * The shell binary contains sentinel-marked placeholder slots:
 *   [32-byte sentinel][4-byte LE actual_length=0][zero padding to max_size]
 *
 * This function locates each sentinel, writes the actual data length and content,
 * and produces a self-contained binary with all assets embedded.
 */
export function patchRunnerBinary(shellPath: string, outputPath: string, input: PatchInput): void {
  const binary = readFileSync(shellPath);

  patchSlot(binary, SENTINELS.assetBundle, input.assetBundle, 'asset bundle');
  patchSlot(binary, SENTINELS.backendBundle, input.backendBundle, 'backend bundle');
  patchSlot(binary, SENTINELS.runnerConfig, input.runnerConfig, 'runner config');

  writeFileSync(outputPath, binary);
}

function patchSlot(binary: Buffer, sentinel: Buffer, data: Buffer, label: string): void {
  const offset = findSentinel(binary, sentinel);
  if (offset === -1) {
    throw new Error(
      `Sentinel for ${label} not found in the shell binary. ` +
      'The binary may not be a valid pre-built shell, or the sentinel was already patched.',
    );
  }

  // The total slot size is from the sentinel to the next sentinel (or end of embedded data).
  // We know the max slot size from the build.rs constants, but we can also compute it:
  // slot = HEADER_SIZE + max_data_size. The placeholder was written as all zeros after the header.
  // We just need to ensure data fits: data.length must be <= (slot_size - HEADER_SIZE).
  const lengthOffset = offset + 32;

  // Read the current actual_length to verify this is an unpatched slot
  const currentLength = binary.readUInt32LE(lengthOffset);
  if (currentLength !== 0) {
    throw new Error(
      `Sentinel slot for ${label} already has data (length=${currentLength}). ` +
      'The binary appears to have been patched already.',
    );
  }

  // Find the end of the zero-padded slot by scanning for non-zero bytes
  // after a reasonable header region. Actually, we know the exact sizes from build.rs:
  // asset=64MB, backend=4MB, config=256KB. But for robustness, just verify size fits.
  const maxSizes: Record<string, number> = {
    'asset bundle': 64 * 1024 * 1024,
    'backend bundle': 4 * 1024 * 1024,
    'runner config': 256 * 1024,
  };
  const maxSize = maxSizes[label];
  if (!maxSize) {
    throw new Error(`Unknown slot label: ${label}`);
  }

  if (data.length > maxSize) {
    throw new Error(
      `${label} is too large to patch: ${data.length} bytes > ${maxSize} byte slot. ` +
      'Use the Cargo compilation path for large applications.',
    );
  }

  // Write actual_length
  binary.writeUInt32LE(data.length, lengthOffset);

  // Write data after the header
  data.copy(binary, offset + HEADER_SIZE);
}

function findSentinel(binary: Buffer, sentinel: Buffer): number {
  return binary.indexOf(sentinel);
}
