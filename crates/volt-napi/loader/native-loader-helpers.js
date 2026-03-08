/* eslint-disable */
// @ts-nocheck

const EXPECTED_NATIVE_BINDING_VERSION = '0.1.0'

const shouldEnforceVersionCheck = () =>
  process.env.NAPI_RS_ENFORCE_VERSION_CHECK &&
  process.env.NAPI_RS_ENFORCE_VERSION_CHECK !== '0'

const createVersionMismatchError = (bindingPackageVersion) =>
  new Error(
    `Native binding package version mismatch, expected ${EXPECTED_NATIVE_BINDING_VERSION} but got ${bindingPackageVersion}. You can reinstall dependencies to fix this issue.`,
  )

function tryLoadExplicitPath(localRequire, loadErrors, libraryPath) {
  try {
    return localRequire(libraryPath)
  } catch (error) {
    loadErrors.push(error)
    return undefined
  }
}

function tryLoadTarget(localRequire, loadErrors, target) {
  try {
    return localRequire(target.localPath)
  } catch (error) {
    loadErrors.push(error)
  }

  try {
    const binding = localRequire(target.packageName)
    const bindingPackageVersion = localRequire(
      `${target.packageName}/package.json`,
    ).version

    if (
      bindingPackageVersion !== EXPECTED_NATIVE_BINDING_VERSION &&
      shouldEnforceVersionCheck()
    ) {
      throw createVersionMismatchError(bindingPackageVersion)
    }

    return binding
  } catch (error) {
    loadErrors.push(error)
  }

  return undefined
}

module.exports = {
  tryLoadExplicitPath,
  tryLoadTarget,
}
