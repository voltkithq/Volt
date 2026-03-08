import { autoUpdater } from 'voltkit';

interface UpdateCheckOptions {
  url: string;
  currentVersion: string;
}

interface UpdateInfo {
  version: string;
  url: string;
  signature: string;
  sha256: string;
}

export async function checkForUpdate(_options: UpdateCheckOptions): Promise<UpdateInfo | null> {
  const result = await autoUpdater.checkForUpdates();
  if (!result) {
    return null;
  }
  return {
    version: result.version,
    url: result.url,
    signature: result.signature,
    sha256: result.sha256,
  };
}

export async function downloadAndInstall(updateInfo: UpdateInfo): Promise<void> {
  await autoUpdater.downloadUpdate(updateInfo);
  autoUpdater.quitAndInstall();
}

export function cancelDownloadAndInstall(): void {
  throw new Error('cancelDownloadAndInstall is not implemented in dev mode.');
}

