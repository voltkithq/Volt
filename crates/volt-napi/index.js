/* eslint-disable */
// @ts-nocheck

const { loadNativeBinding } = require('./loader/native-loader');
const { isMusl } = require('./loader/musl');
const { loadWasiFallback } = require('./loader/wasi-loader');

const loadErrors = [];

let nativeBinding = loadNativeBinding(loadErrors, require, isMusl);
nativeBinding = loadWasiFallback(nativeBinding, loadErrors, require);

if (!nativeBinding) {
  if (loadErrors.length > 0) {
    throw new Error(
      `Cannot find native binding. ` +
        `npm has a bug related to optional dependencies (https://github.com/npm/cli/issues/4828). ` +
        'Please try `npm i` again after removing both package-lock.json and node_modules directory.',
      {
        cause: loadErrors.reduce((err, cur) => {
          cur.cause = err;
          return cur;
        }),
      },
    );
  }

  throw new Error('Failed to load native binding');
}

module.exports = nativeBinding;
module.exports.VoltApp = nativeBinding.VoltApp;
module.exports.VoltGlobalShortcut = nativeBinding.VoltGlobalShortcut;
module.exports.VoltIpc = nativeBinding.VoltIpc;
module.exports.VoltMenu = nativeBinding.VoltMenu;
module.exports.VoltTray = nativeBinding.VoltTray;
module.exports.clipboardReadImage = nativeBinding.clipboardReadImage;
module.exports.clipboardReadText = nativeBinding.clipboardReadText;
module.exports.clipboardWriteImage = nativeBinding.clipboardWriteImage;
module.exports.clipboardWriteText = nativeBinding.clipboardWriteText;
module.exports.dialogShowMessage = nativeBinding.dialogShowMessage;
module.exports.dialogShowOpen = nativeBinding.dialogShowOpen;
module.exports.dialogShowOpenWithGrant = nativeBinding.dialogShowOpenWithGrant;
module.exports.dialogShowSave = nativeBinding.dialogShowSave;
module.exports.fsCopy = nativeBinding.fsCopy;
module.exports.fsExists = nativeBinding.fsExists;
module.exports.fsMkdir = nativeBinding.fsMkdir;
module.exports.fsReadDir = nativeBinding.fsReadDir;
module.exports.fsReadFile = nativeBinding.fsReadFile;
module.exports.fsReadFileText = nativeBinding.fsReadFileText;
module.exports.fsRemove = nativeBinding.fsRemove;
module.exports.fsRename = nativeBinding.fsRename;
module.exports.fsResolveGrant = nativeBinding.fsResolveGrant;
module.exports.fsStat = nativeBinding.fsStat;
module.exports.fsWatchClose = nativeBinding.fsWatchClose;
module.exports.fsWatchPoll = nativeBinding.fsWatchPoll;
module.exports.fsWatchStart = nativeBinding.fsWatchStart;
module.exports.fsWriteFile = nativeBinding.fsWriteFile;
module.exports.notificationShow = nativeBinding.notificationShow;
module.exports.shellOpenExternal = nativeBinding.shellOpenExternal;
module.exports.shellShowItemInFolder = nativeBinding.shellShowItemInFolder;
module.exports.updaterApply = nativeBinding.updaterApply;
module.exports.updaterCheck = nativeBinding.updaterCheck;
module.exports.updaterDownloadAndVerify = nativeBinding.updaterDownloadAndVerify;
module.exports.windowClose = nativeBinding.windowClose;
module.exports.windowCount = nativeBinding.windowCount;
module.exports.windowEvalScript = nativeBinding.windowEvalScript;
module.exports.windowFocus = nativeBinding.windowFocus;
module.exports.windowMaximize = nativeBinding.windowMaximize;
module.exports.windowMinimize = nativeBinding.windowMinimize;
module.exports.windowRestore = nativeBinding.windowRestore;
module.exports.windowShow = nativeBinding.windowShow;
