declare module 'volt:clipboard' {
  export function readText(): string;
  export function writeText(text: string): void;
}

declare module 'volt:crypto' {
  export function sha256(data: string): string;
  export function base64Encode(data: string): string;
  export function base64Decode(data: string): string;
}

declare module 'volt:os' {
  export function platform(): string;
  export function arch(): string;
  export function homeDir(): string;
  export function tempDir(): string;
}

declare module 'volt:shell' {
  export function openExternal(url: string): Promise<void>;
  export function showItemInFolder(path: string): void;
}
