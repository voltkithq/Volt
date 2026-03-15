import type { DoctorCheckContext, DoctorCheckResult, DoctorDeps } from './types.js';
import {
  collectMacSigningChecks,
  collectWindowsSigningChecks,
  createToolCheck,
} from './signing-checks.js';

export function collectDoctorChecks(
  context: DoctorCheckContext,
  deps: DoctorDeps,
): DoctorCheckResult[] {
  const checks: DoctorCheckResult[] = [
    createToolCheck(
      'tool.cargo',
      'Rust toolchain (`cargo`)',
      'cargo',
      deps.isToolAvailable,
      'required for `volt build` and packaging workflows',
    ),
    createToolCheck(
      'tool.rustc',
      'Rust compiler (`rustc`)',
      'rustc',
      deps.isToolAvailable,
      'required for native runtime compilation',
    ),
  ];

  if (context.platform === 'win32') {
    if (context.formats.includes('nsis')) {
      checks.push(
        createToolCheck(
          'pkg.win.nsis',
          'NSIS packager (`makensis`)',
          'makensis',
          deps.isToolAvailable,
          'required for NSIS installer output',
        ),
      );
    }

    if (context.formats.includes('msix')) {
      const hasMakemsix = deps.isToolAvailable('makemsix');
      const hasMakeappx = deps.isToolAvailable('makeappx');
      checks.push({
        id: 'pkg.win.msix',
        status: hasMakemsix || hasMakeappx ? 'pass' : 'fail',
        title: 'MSIX packager (`makemsix` or `makeappx`)',
        details:
          hasMakemsix || hasMakeappx
            ? 'MSIX packaging tools detected'
            : 'install Windows SDK tooling (`makemsix` or `makeappx`) to build MSIX packages',
      });
    }
  }

  if (context.platform === 'darwin' && context.formats.includes('dmg')) {
    checks.push(
      createToolCheck(
        'pkg.mac.dmg',
        'DMG tool (`hdiutil`)',
        'hdiutil',
        deps.isToolAvailable,
        'required for `.dmg` output',
      ),
    );
  }

  if (context.platform === 'linux') {
    if (context.formats.includes('appimage')) {
      checks.push(
        createToolCheck(
          'pkg.linux.appimage',
          'AppImage tool (`appimagetool`)',
          'appimagetool',
          deps.isToolAvailable,
          'required for `.AppImage` output',
        ),
      );
    }
    if (context.formats.includes('deb')) {
      checks.push(
        createToolCheck(
          'pkg.linux.deb',
          'Debian packager (`dpkg-deb`)',
          'dpkg-deb',
          deps.isToolAvailable,
          'required for `.deb` output',
        ),
      );
    }
  }

  if (context.platform === 'win32') {
    checks.push(...collectWindowsSigningChecks(context, deps));
  }
  if (context.platform === 'darwin') {
    checks.push(...collectMacSigningChecks(context, deps));
  }

  return checks;
}
