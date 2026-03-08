# Clipboard

System clipboard read/write. Requires `permissions: ['clipboard']`.

## `clipboard.readText(): string`

Read text from the system clipboard.

```ts
import { clipboard } from 'voltkit';

const text = clipboard.readText();
```

## `clipboard.writeText(text: string): void`

Write text to the system clipboard.

```ts
clipboard.writeText('Hello from Volt!');
```

## `clipboard.readImage(): ClipboardImage | null`

Read an image from the system clipboard. Returns `null` if no image is available.

```ts
const img = clipboard.readImage();
if (img) {
  console.log(`${img.width}x${img.height}, ${img.rgba.length} bytes`);
}
```

## `clipboard.writeImage(image: ClipboardImage): void`

Write an image to the system clipboard.

```ts
clipboard.writeImage({
  rgba: new Uint8Array([255, 0, 0, 255]), // 1x1 red pixel
  width: 1,
  height: 1,
});
```

## `ClipboardImage`

```ts
interface ClipboardImage {
  rgba: Uint8Array;  // RGBA pixel bytes
  width: number;     // Image width in pixels
  height: number;    // Image height in pixels
}
```

**Note:** The native layer enforces a maximum image size of 10 MB to prevent out-of-memory conditions.
