/* eslint-disable */
// @ts-nocheck

const createTarget = (suffix) => ({
  localPath: `./volt-native.${suffix}.node`,
  packageName: `@voltkit/volt-native-${suffix}`,
})

const TARGETS = {
  android: {
    arm64: createTarget('android-arm64'),
    arm: createTarget('android-arm-eabi'),
  },
  win32: {
    x64: {
      gnu: createTarget('win32-x64-gnu'),
      msvc: createTarget('win32-x64-msvc'),
    },
    ia32: createTarget('win32-ia32-msvc'),
    arm64: createTarget('win32-arm64-msvc'),
  },
  darwin: {
    universal: createTarget('darwin-universal'),
    x64: createTarget('darwin-x64'),
    arm64: createTarget('darwin-arm64'),
  },
  freebsd: {
    x64: createTarget('freebsd-x64'),
    arm64: createTarget('freebsd-arm64'),
  },
  linux: {
    x64: {
      gnu: createTarget('linux-x64-gnu'),
      musl: createTarget('linux-x64-musl'),
    },
    arm64: {
      gnu: createTarget('linux-arm64-gnu'),
      musl: createTarget('linux-arm64-musl'),
    },
    arm: {
      gnu: createTarget('linux-arm-gnueabihf'),
      musl: createTarget('linux-arm-musleabihf'),
    },
    loong64: {
      gnu: createTarget('linux-loong64-gnu'),
      musl: createTarget('linux-loong64-musl'),
    },
    riscv64: {
      gnu: createTarget('linux-riscv64-gnu'),
      musl: createTarget('linux-riscv64-musl'),
    },
    ppc64: {
      default: createTarget('linux-ppc64-gnu'),
    },
    s390x: {
      default: createTarget('linux-s390x-gnu'),
    },
  },
  openharmony: {
    arm64: createTarget('openharmony-arm64'),
    x64: createTarget('openharmony-x64'),
    arm: createTarget('openharmony-arm'),
  },
}

const isWindowsGnuRuntime = () =>
  process.config?.variables?.shlib_suffix === 'dll.a' ||
  process.config?.variables?.node_target_type === 'shared_library'

const resolveDarwinTargets = (arch) => {
  const targets = [TARGETS.darwin.universal]

  if (arch === 'x64') {
    targets.push(TARGETS.darwin.x64)
    return { targets }
  }

  if (arch === 'arm64') {
    targets.push(TARGETS.darwin.arm64)
    return { targets }
  }

  return {
    targets,
    unsupportedError: new Error(`Unsupported architecture on macOS: ${arch}`),
  }
}

const resolveLinuxTarget = (arch, isMusl) => {
  const variant = TARGETS.linux[arch]
  if (!variant) {
    return {
      targets: [],
      unsupportedError: new Error(`Unsupported architecture on Linux: ${arch}`),
    }
  }

  if (variant.default) {
    return { targets: [variant.default] }
  }

  const target = isMusl() ? variant.musl : variant.gnu
  return { targets: [target] }
}

function resolveNativeTargets(isMusl) {
  const platform = process.platform
  const arch = process.arch

  if (platform === 'android') {
    const target = TARGETS.android[arch]
    if (!target) {
      return {
        targets: [],
        unsupportedError: new Error(`Unsupported architecture on Android ${arch}`),
      }
    }

    return { targets: [target] }
  }

  if (platform === 'win32') {
    if (arch === 'x64') {
      return {
        targets: [
          isWindowsGnuRuntime() ? TARGETS.win32.x64.gnu : TARGETS.win32.x64.msvc,
        ],
      }
    }

    const target = TARGETS.win32[arch]
    if (!target) {
      return {
        targets: [],
        unsupportedError: new Error(`Unsupported architecture on Windows: ${arch}`),
      }
    }

    return { targets: [target] }
  }

  if (platform === 'darwin') {
    return resolveDarwinTargets(arch)
  }

  if (platform === 'freebsd') {
    const target = TARGETS.freebsd[arch]
    if (!target) {
      return {
        targets: [],
        unsupportedError: new Error(`Unsupported architecture on FreeBSD: ${arch}`),
      }
    }

    return { targets: [target] }
  }

  if (platform === 'linux') {
    return resolveLinuxTarget(arch, isMusl)
  }

  if (platform === 'openharmony') {
    const target = TARGETS.openharmony[arch]
    if (!target) {
      return {
        targets: [],
        unsupportedError: new Error(`Unsupported architecture on OpenHarmony: ${arch}`),
      }
    }

    return { targets: [target] }
  }

  return {
    targets: [],
    unsupportedError: new Error(
      `Unsupported OS: ${platform}, architecture: ${arch}`,
    ),
  }
}

module.exports = {
  resolveNativeTargets,
}
