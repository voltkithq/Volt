export interface UpdatePublishOptions {
  artifactsDir?: string;
  outDir?: string;
  provider?: string;
  channel?: string;
  baseUrl?: string;
  manifestFile?: string;
  dryRun?: boolean;
}

export interface PublishArtifactRecord {
  fileName: string;
  sha256: string;
  size: number;
  url: string;
}

export interface UpdateReleaseManifest {
  schemaVersion: 1;
  appName: string;
  channel: string;
  generatedAt: string;
  update: {
    version: string;
    url: string;
    signature: string;
    sha256: string;
  };
  artifacts: PublishArtifactRecord[];
}

export interface PublishProvider {
  name: string;
  publishArtifact(
    sourceAbsolutePath: string,
    destinationFileName: string,
  ): Promise<{ location: string }>;
  publishManifest(
    manifestJson: string,
    manifestFileName: string,
  ): Promise<{ location: string }>;
}
