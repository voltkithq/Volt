/* eslint-disable */
// @ts-nocheck

function loadWasiFallback(nativeBinding, loadErrors, localRequire) {
  if (!nativeBinding || process.env.NAPI_RS_FORCE_WASI) {
    let wasiBinding = null
    let wasiBindingError = null
    try {
      wasiBinding = localRequire('./volt-native.wasi.cjs')
      nativeBinding = wasiBinding
    } catch (err) {
      if (process.env.NAPI_RS_FORCE_WASI) {
        wasiBindingError = err
      }
    }
    if (!nativeBinding || process.env.NAPI_RS_FORCE_WASI) {
      try {
        wasiBinding = localRequire('@voltkit/volt-native-wasm32-wasi')
        nativeBinding = wasiBinding
      } catch (err) {
        if (process.env.NAPI_RS_FORCE_WASI) {
          if (!wasiBindingError) {
            wasiBindingError = err
          } else {
            wasiBindingError.cause = err
          }
          loadErrors.push(err)
        }
      }
    }
    if (process.env.NAPI_RS_FORCE_WASI === 'error' && !wasiBinding) {
      const error = new Error('WASI binding not found and NAPI_RS_FORCE_WASI is set to error')
      error.cause = wasiBindingError
      throw error
    }
  }

  return nativeBinding
}

module.exports = {
  loadWasiFallback,
}
