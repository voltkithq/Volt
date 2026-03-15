import type { DoctorPlatform } from './types.js';

export function resolveDoctorFormats(
  platform: DoctorPlatform,
  requestedFormat: string | undefined,
): string[] {
  if (requestedFormat) {
    return [requestedFormat];
  }
  if (platform === 'win32') {
    return ['nsis'];
  }
  if (platform === 'darwin') {
    return ['app'];
  }
  return ['appimage', 'deb'];
}
