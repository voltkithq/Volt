import { escapeNsisString, escapeXml } from './helpers.js';
import type { PackageConfig, WindowsInstallMode } from './types.js';

export interface NsisScriptOptions {
  installMode?: WindowsInstallMode;
  silentAllUsers?: boolean;
}

export interface MsixManifestOptions {
  identityName: string;
  publisher: string;
  publisherDisplayName: string;
  displayName: string;
  description: string;
  executableFileName: string;
  version: string;
  square44Logo: string;
  square150Logo: string;
}

export function generateNsisScript(
  appName: string,
  artifactVersion: string,
  installerBinaryName: string,
  distDir: string,
  outDir: string,
  runtimeExecutableFileName: string,
  additionalFileNames: string[] = [],
  options: NsisScriptOptions = {},
): string {
  const installMode = options.installMode ?? 'perMachine';
  const isPerMachine = installMode === 'perMachine';
  const defaultAllUsers = isPerMachine ? '1' : '0';
  const defaultInstallDir = isPerMachine
    ? '$PROGRAMFILES64'
    : '$LOCALAPPDATA';
  const requestExecutionLevel = isPerMachine ? 'admin' : 'user';
  const escapedAppName = escapeNsisString(appName);
  const escapedArtifactVersion = escapeNsisString(artifactVersion);
  const escapedDistDir = escapeNsisString(distDir);
  const escapedOutDir = escapeNsisString(outDir);
  const escapedInstallerBinaryName = escapeNsisString(installerBinaryName);
  const escapedRuntimeExecutableFileName = escapeNsisString(runtimeExecutableFileName);
  const registryKey = escapeNsisString(`Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${installerBinaryName}`);
  const additionalInstallFiles = additionalFileNames
    .map((fileName) => `  File "${escapedDistDir}\\${escapeNsisString(fileName)}"`)
    .join('\n');
  const additionalUninstallFiles = additionalFileNames
    .map((fileName) => `  Delete "$INSTDIR\\${escapeNsisString(fileName)}"`)
    .join('\n');
  const allUsersInit = isPerMachine
    ? [
      '  ${GetParameters} $R0',
      '  ${GetOptions} $R0 "/ALLUSERS=" $R1',
      '  ${If} $R1 == "1"',
      '    StrCpy $VoltAllUsers "1"',
      '  ${ElseIf} $R1 == "0"',
      '    StrCpy $VoltAllUsers "0"',
      '  ${EndIf}',
      ...(options.silentAllUsers === false ? [] : [
        '  IfSilent 0 +2',
        '  StrCpy $VoltAllUsers "1"',
      ]),
    ].join('\n')
    : '';

  return `
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"

Name "${escapedAppName}"
OutFile "${escapedOutDir}\\${escapedInstallerBinaryName}-${escapedArtifactVersion}-setup.exe"
InstallDir "${defaultInstallDir}\\${escapedAppName}"
RequestExecutionLevel ${requestExecutionLevel}

Var VoltAllUsers

Function .onInit
  StrCpy $VoltAllUsers "${defaultAllUsers}"
${allUsersInit}
FunctionEnd

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  ${isPerMachine ? '${If} $VoltAllUsers == "1"' : '${If} 0 == 1'}
    SetShellVarContext all
    StrCpy $INSTDIR "$PROGRAMFILES64\\${escapedAppName}"
  ${'${Else}'}
    SetShellVarContext current
    StrCpy $INSTDIR "$LOCALAPPDATA\\${escapedAppName}"
  ${'${EndIf}'}
  SetOutPath $INSTDIR
  File "${escapedDistDir}\\${escapedRuntimeExecutableFileName}"
${additionalInstallFiles}
  CreateShortCut "$DESKTOP\\${escapedAppName}.lnk" "$INSTDIR\\${escapedRuntimeExecutableFileName}"
  CreateShortCut "$SMPROGRAMS\\${escapedAppName}.lnk" "$INSTDIR\\${escapedRuntimeExecutableFileName}"
  WriteUninstaller "$INSTDIR\\uninstall.exe"
  ${isPerMachine ? '${If} $VoltAllUsers == "1"' : '${If} 0 == 1'}
    WriteRegStr HKLM "${registryKey}" "DisplayName" "${escapedAppName}"
    WriteRegStr HKLM "${registryKey}" "UninstallString" '"$INSTDIR\\uninstall.exe" /S /ALLUSERS=1'
  ${'${Else}'}
    WriteRegStr HKCU "${registryKey}" "DisplayName" "${escapedAppName}"
    WriteRegStr HKCU "${registryKey}" "UninstallString" '"$INSTDIR\\uninstall.exe" /S /ALLUSERS=0'
  ${'${EndIf}'}
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\\${escapedRuntimeExecutableFileName}"
${additionalUninstallFiles}
  Delete "$INSTDIR\\uninstall.exe"
  SetShellVarContext all
  Delete "$DESKTOP\\${escapedAppName}.lnk"
  Delete "$SMPROGRAMS\\${escapedAppName}.lnk"
  SetShellVarContext current
  Delete "$DESKTOP\\${escapedAppName}.lnk"
  Delete "$SMPROGRAMS\\${escapedAppName}.lnk"
  DeleteRegKey HKLM "${registryKey}"
  DeleteRegKey HKCU "${registryKey}"
  RMDir "$INSTDIR"
SectionEnd
`.trim();
}

export function generateMsixManifest(options: MsixManifestOptions): string {
  const escapedIdentityName = escapeXml(options.identityName);
  const escapedPublisher = escapeXml(options.publisher);
  const escapedPublisherDisplayName = escapeXml(options.publisherDisplayName);
  const escapedDisplayName = escapeXml(options.displayName);
  const escapedDescription = escapeXml(options.description);
  const escapedExecutable = escapeXml(options.executableFileName);
  const escapedVersion = escapeXml(options.version);
  const escapedSquare44Logo = escapeXml(options.square44Logo);
  const escapedSquare150Logo = escapeXml(options.square150Logo);

  return `<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:desktop="http://schemas.microsoft.com/appx/manifest/desktop/windows10"
  IgnorableNamespaces="uap desktop">
  <Identity
    Name="${escapedIdentityName}"
    Publisher="${escapedPublisher}"
    Version="${escapedVersion}" />
  <Properties>
    <DisplayName>${escapedDisplayName}</DisplayName>
    <PublisherDisplayName>${escapedPublisherDisplayName}</PublisherDisplayName>
    <Description>${escapedDescription}</Description>
    <Logo>${escapedSquare150Logo}</Logo>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily
      Name="Windows.Desktop"
      MinVersion="10.0.17763.0"
      MaxVersionTested="10.0.22621.0" />
  </Dependencies>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Applications>
    <Application Id="App" Executable="${escapedExecutable}" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="${escapedDisplayName}"
        Description="${escapedDescription}"
        BackgroundColor="transparent"
        Square44x44Logo="${escapedSquare44Logo}"
        Square150x150Logo="${escapedSquare150Logo}" />
      <Extensions>
        <desktop:Extension Category="windows.fullTrustProcess" Executable="${escapedExecutable}" />
      </Extensions>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities" />
  </Capabilities>
</Package>`;
}

export function generateAppRun(binaryName: string): string {
  return `#!/bin/bash\nHERE="$(dirname "$(readlink -f "${0}")")"\nexec "$HERE/usr/bin/${binaryName}" "$@"\n`;
}

export function generateInfoPlist(
  appName: string,
  version: string,
  binaryName: string,
  config: PackageConfig,
): string {
  const escapedAppName = escapeXml(appName);
  const escapedVersion = escapeXml(version);
  const escapedBinaryName = escapeXml(binaryName);
  const escapedIdentifier = escapeXml(config.identifier);
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${escapedAppName}</string>
  <key>CFBundleDisplayName</key>
  <string>${escapedAppName}</string>
  <key>CFBundleIdentifier</key>
  <string>${escapedIdentifier}</string>
  <key>CFBundleVersion</key>
  <string>${escapedVersion}</string>
  <key>CFBundleShortVersionString</key>
  <string>${escapedVersion}</string>
  <key>CFBundleExecutable</key>
  <string>${escapedBinaryName}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>`;
}

export function generateDesktopFile(
  appName: string,
  binaryName: string,
  config: PackageConfig,
  execCommand = binaryName,
): string {
  const categories = config.categories?.join(';') ?? 'Utility';
  return `[Desktop Entry]
Name=${appName}
Exec=${execCommand}
Terminal=false
Type=Application
Icon=${binaryName}
Categories=${categories};
`;
}
