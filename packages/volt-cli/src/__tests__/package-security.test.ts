import { describe, it, expect } from 'vitest';
import { __testOnly } from '../commands/package.js';
import { toSafeBinaryName } from '../utils/naming.js';

describe('package NSIS security hardening', () => {
  it('escapes NSIS-sensitive characters from interpolated strings', () => {
    const escaped = __testOnly.escapeNsisString('My "App"\n$PROGRAMFILES');
    expect(escaped).toContain('$\\"');
    expect(escaped).toContain('$$PROGRAMFILES');
    expect(escaped).not.toContain('\n');
  });

  it('does not allow multiline NSIS directive injection via app name', () => {
    const script = __testOnly.generateNsisScript(
      'Volt"\n!define PWNED 1',
      '1.0.0',
      'volt-app',
      'C:\\dist',
      'C:\\dist-package',
      'volt-app.exe',
    );

    expect(script).not.toContain('\n!define PWNED 1');
    expect(script).toContain('Name "Volt$\\" !define PWNED 1"');
  });

  it('normalizes malicious app names into safe binary stems', () => {
    expect(toSafeBinaryName('../../evil";rm -rf /')).toBe('evil-rm-rf');
  });

  it('AppRun uses sanitized binary names', () => {
    const appRun = __testOnly.generateAppRun(toSafeBinaryName('My App";touch /tmp/pwn'));
    expect(appRun).toContain('exec "$HERE/usr/bin/my-app-touch-tmp-pwn" "$@"');
    expect(appRun).not.toContain(';touch');
  });

  it('escapes XML values in generated plist fields', () => {
    const escaped = __testOnly.escapeXml(`A&B <C> "D" 'E'`);
    expect(escaped).toBe('A&amp;B &lt;C&gt; &quot;D&quot; &apos;E&apos;');
  });

  it('writes NSIS installer output under the package directory', () => {
    const script = __testOnly.generateNsisScript(
      'Volt App',
      '1.0.0',
      'volt-app',
      'C:\\dist',
      'C:\\dist-package',
      'volt-app.exe',
    );
    expect(script).toContain('OutFile "C:\\dist-package\\volt-app-1.0.0-setup.exe"');
  });

  it('includes updater helper install and uninstall entries when provided', () => {
    const script = __testOnly.generateNsisScript(
      'Volt App',
      '1.0.0',
      'volt-app',
      'C:\\dist',
      'C:\\dist-package',
      'volt-app.exe',
      ['volt-updater-helper.exe'],
    );

    expect(script).toContain('File "C:\\dist\\volt-updater-helper.exe"');
    expect(script).toContain('Delete "$INSTDIR\\volt-updater-helper.exe"');
  });

  it('uses explicit AppRun exec command for AppImage desktop entries', () => {
    const desktop = __testOnly.generateDesktopFile('Volt App', 'volt-app', { identifier: 'com.volt.test' }, 'AppRun');
    expect(desktop).toContain('\nExec=AppRun\n');
  });

  it('writes a desktop icon entry for Linux package metadata', () => {
    const desktop = __testOnly.generateDesktopFile('Volt App', 'volt-app', { identifier: 'com.volt.test' });
    expect(desktop).toContain('\nIcon=volt-app\n');
  });

  it('normalizes Debian control version strings to safe characters', () => {
    expect(__testOnly.normalizeDebianControlVersion(' 1.0.0 beta/rc ')).toBe('1.0.0-beta-rc');
    expect(__testOnly.normalizeDebianControlVersion('!!!')).toBe('0.1.0');
  });

  it('validates requested package format by platform', () => {
    expect(__testOnly.validateRequestedPackageFormat('linux', 'deb')).toBe('deb');
    expect(__testOnly.validateRequestedPackageFormat('darwin', 'DMG')).toBe('dmg');
    expect(__testOnly.validateRequestedPackageFormat('win32', 'deb')).toBeUndefined();
    expect(__testOnly.validateRequestedPackageFormat('win32', 'msix')).toBe('msix');
  });

  it('normalizes Windows install mode aliases', () => {
    expect(__testOnly.normalizeWindowsInstallMode('perMachine')).toBe('perMachine');
    expect(__testOnly.normalizeWindowsInstallMode('per-machine')).toBe('perMachine');
    expect(__testOnly.normalizeWindowsInstallMode('PerUser')).toBe('perUser');
    expect(__testOnly.normalizeWindowsInstallMode('per-user')).toBe('perUser');
    expect(__testOnly.normalizeWindowsInstallMode('system')).toBeUndefined();
  });

  it('renders per-user NSIS scripts with non-admin execution level', () => {
    const script = __testOnly.generateNsisScript(
      'Volt App',
      '1.0.0',
      'volt-app',
      'C:\\dist',
      'C:\\dist-package',
      'volt-app.exe',
      [],
      { installMode: 'perUser' },
    );

    expect(script).toContain('RequestExecutionLevel user');
    expect(script).toContain('InstallDir "$LOCALAPPDATA\\Volt App"');
    expect(script).not.toContain('GetOptions $R0 "/ALLUSERS=" $R1');
  });

  it('supports disabling forced all-users in silent per-machine installs', () => {
    const script = __testOnly.generateNsisScript(
      'Volt App',
      '1.0.0',
      'volt-app',
      'C:\\dist',
      'C:\\dist-package',
      'volt-app.exe',
      [],
      { installMode: 'perMachine', silentAllUsers: false },
    );

    expect(script).toContain('RequestExecutionLevel admin');
    expect(script).not.toContain('IfSilent 0 +2');
  });

  it('normalizes MSIX versions to four numeric parts', () => {
    expect(__testOnly.normalizeMsixVersion('1.2.3')).toBe('1.2.3.0');
    expect(__testOnly.normalizeMsixVersion('2.4.6.8.9')).toBe('2.4.6.8');
    expect(__testOnly.normalizeMsixVersion('7.1.0-beta.5')).toBe('7.1.0.5');
  });

  it('escapes XML-sensitive values in generated MSIX manifests', () => {
    const manifest = __testOnly.generateMsixManifest({
      identityName: 'com.example.test',
      publisher: 'CN=Acme & Co',
      publisherDisplayName: 'Acme <Team>',
      displayName: 'Volt "App"',
      description: `A&B app`,
      executableFileName: 'volt-app.exe',
      version: '1.0.0.0',
      square44Logo: 'Assets/Square44x44Logo.png',
      square150Logo: 'Assets/Square150x150Logo.png',
    });

    expect(manifest).toContain('CN=Acme &amp; Co');
    expect(manifest).toContain('Acme &lt;Team&gt;');
    expect(manifest).toContain('Volt &quot;App&quot;');
    expect(manifest).toContain('A&amp;B app');
  });
});
