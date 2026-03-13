import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Convert a PNG file to ICO format suitable for Windows resource embedding.
 *
 * ICO is a simple container: a 6-byte header, one 16-byte directory entry per
 * image, then the raw PNG data. Modern Windows (Vista+) supports PNG-compressed
 * ICO entries, so we embed the PNG directly without re-encoding to BMP.
 */
export function convertPngToIco(pngPath: string, outDir: string): string {
  const png = readFileSync(pngPath);
  validatePngSignature(png);
  const { width, height } = readPngDimensions(png);

  // ICO header (6 bytes)
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: 1 = ICO
  header.writeUInt16LE(1, 4); // image count: 1

  // Directory entry (16 bytes)
  const entry = Buffer.alloc(16);
  entry.writeUInt8(width >= 256 ? 0 : width, 0); // width (0 = 256)
  entry.writeUInt8(height >= 256 ? 0 : height, 1); // height (0 = 256)
  entry.writeUInt8(0, 2); // color palette count
  entry.writeUInt8(0, 3); // reserved
  entry.writeUInt16LE(1, 4); // color planes
  entry.writeUInt16LE(32, 6); // bits per pixel
  entry.writeUInt32LE(png.length, 8); // image data size
  entry.writeUInt32LE(6 + 16, 12); // offset to image data (after header + 1 entry)

  const ico = Buffer.concat([header, entry, png]);
  const icoPath = resolve(outDir, 'app-icon.ico');
  writeFileSync(icoPath, ico);
  return icoPath;
}

function validatePngSignature(data: Buffer): void {
  const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (data.length < 8 || !data.subarray(0, 8).equals(PNG_SIGNATURE)) {
    throw new Error('File is not a valid PNG image.');
  }
}

function readPngDimensions(data: Buffer): { width: number; height: number } {
  // IHDR chunk starts at byte 8 (4 bytes length + 4 bytes "IHDR" + 4 bytes width + 4 bytes height)
  if (data.length < 24) {
    throw new Error('PNG file is too small to contain valid dimensions.');
  }
  const width = data.readUInt32BE(16);
  const height = data.readUInt32BE(20);
  return { width, height };
}
