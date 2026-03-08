/* eslint-disable */
// @ts-nocheck

const { tryLoadExplicitPath, tryLoadTarget } = require('./native-loader-helpers')
const { resolveNativeTargets } = require('./native-loader-targets')

function requireNative(localRequire, loadErrors, isMusl) {
  if (process.env.NAPI_RS_NATIVE_LIBRARY_PATH) {
    return tryLoadExplicitPath(
      localRequire,
      loadErrors,
      process.env.NAPI_RS_NATIVE_LIBRARY_PATH,
    )
  }

  const { targets, unsupportedError } = resolveNativeTargets(isMusl)

  for (const target of targets) {
    const binding = tryLoadTarget(localRequire, loadErrors, target)
    if (binding) {
      return binding
    }
  }

  if (unsupportedError) {
    loadErrors.push(unsupportedError)
  }
}

module.exports = {
  requireNative,
}
