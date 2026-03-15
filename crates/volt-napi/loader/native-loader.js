/* eslint-disable */
// @ts-nocheck

const { resolveNativeTargets } = require('./native-loader-targets');

const EXPECTED_BINDING_VERSION = '0.1.10';

const shouldEnforceVersionCheck = () =>
  process.env.NAPI_RS_ENFORCE_VERSION_CHECK && process.env.NAPI_RS_ENFORCE_VERSION_CHECK !== '0';

const createVersionError = (actualVersion) =>
  new Error(
    `Native binding package version mismatch, expected ${EXPECTED_BINDING_VERSION} but got ${actualVersion}. You can reinstall dependencies to fix this issue.`,
  );

function loadTargetBinding(target, loadErrors, localRequire) {
  try {
    return localRequire(target.localPath);
  } catch (error) {
    loadErrors.push(error);
  }

  try {
    const binding = localRequire(target.packageName);
    const bindingPackageVersion = localRequire(`${target.packageName}/package.json`).version;
    if (shouldEnforceVersionCheck() && bindingPackageVersion !== EXPECTED_BINDING_VERSION) {
      throw createVersionError(bindingPackageVersion);
    }
    return binding;
  } catch (error) {
    loadErrors.push(error);
  }

  return null;
}

function loadNativeBinding(loadErrors, localRequire, isMusl) {
  if (process.env.NAPI_RS_NATIVE_LIBRARY_PATH) {
    try {
      return localRequire(process.env.NAPI_RS_NATIVE_LIBRARY_PATH);
    } catch (error) {
      loadErrors.push(error);
    }
  }

  const { targets, unsupportedError } = resolveNativeTargets(isMusl);
  for (const target of targets) {
    const binding = loadTargetBinding(target, loadErrors, localRequire);
    if (binding) {
      return binding;
    }
  }

  if (targets.length === 0 && unsupportedError) {
    loadErrors.push(unsupportedError);
  }

  return null;
}

module.exports = {
  loadNativeBinding,
};
