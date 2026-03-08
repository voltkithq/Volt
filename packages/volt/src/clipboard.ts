/**
 * Clipboard module.
 * Provides read/write access to the system clipboard.
 * Requires `permissions: ['clipboard']` in volt.config.ts.
 */

import {
  clipboardReadText,
  clipboardWriteText,
  clipboardReadImage,
  clipboardWriteImage,
} from '@voltkit/volt-native';

/** Image data from the clipboard. */
export interface ClipboardImage {
  /** RGBA pixel bytes. */
  rgba: Uint8Array;
  /** Image width in pixels. */
  width: number;
  /** Image height in pixels. */
  height: number;
}

/**
 * Read text from the system clipboard.
 *
 * @example
 * ```ts
 * const text = clipboard.readText();
 * ```
 */
function readText(): string {
  return clipboardReadText();
}

/** Write text to the system clipboard. */
function writeText(text: string): void {
  clipboardWriteText(text);
}

/** Read an image from the system clipboard. Returns null if no image. */
function readImage(): ClipboardImage | null {
  try {
    const img = clipboardReadImage();
    return {
      rgba: new Uint8Array(img.rgba),
      width: img.width,
      height: img.height,
    };
  } catch (err) {
    if (isNoImageError(err)) {
      return null;
    }
    throw err;
  }
}

/** Write an image to the system clipboard. */
function writeImage(image: ClipboardImage): void {
  clipboardWriteImage({
    rgba: Buffer.from(image.rgba),
    width: image.width,
    height: image.height,
  });
}

function isNoImageError(err: unknown): boolean {
  if (!(err instanceof Error)) {
    return false;
  }
  const message = err.message.toLowerCase();
  return message.includes('no image')
    || message.includes('content not available')
    || message.includes('contentnotavailable')
    || message.includes('clipboard is empty');
}

/** Clipboard APIs. Requires `permissions: ['clipboard']` in volt.config.ts. */
export const clipboard = {
  readText,
  writeText,
  readImage,
  writeImage,
};
