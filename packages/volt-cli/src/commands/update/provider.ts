import { copyFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { PublishProvider } from './types.js';

class LocalFilesystemPublishProvider implements PublishProvider {
  public readonly name = 'local';
  private readonly rootDir: string;
  private readonly dryRun: boolean;

  constructor(rootDir: string, dryRun: boolean) {
    this.rootDir = rootDir;
    this.dryRun = dryRun;
  }

  async publishArtifact(
    sourceAbsolutePath: string,
    destinationFileName: string,
  ): Promise<{ location: string }> {
    const destination = resolve(this.rootDir, destinationFileName);
    if (!this.dryRun) {
      mkdirSync(this.rootDir, { recursive: true });
      copyFileSync(sourceAbsolutePath, destination);
    }
    return { location: destination };
  }

  async publishManifest(
    manifestJson: string,
    manifestFileName: string,
  ): Promise<{ location: string }> {
    const destination = resolve(this.rootDir, manifestFileName);
    if (!this.dryRun) {
      mkdirSync(this.rootDir, { recursive: true });
      writeFileSync(destination, manifestJson, 'utf8');
    }
    return { location: destination };
  }
}

export function createPublishProvider(
  providerName: string,
  rootDir: string,
  dryRun: boolean,
): PublishProvider {
  const normalized = providerName.trim().toLowerCase();
  if (normalized === 'local' || normalized === '') {
    return new LocalFilesystemPublishProvider(rootDir, dryRun);
  }
  throw new Error(`[volt] Unsupported publish provider "${providerName}". Supported: local`);
}
