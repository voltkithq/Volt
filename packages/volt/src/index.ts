import './runtime-modules.js';

export { VoltApp, createApp, getApp } from './app.js';
export { BrowserWindow } from './window.js';
export { ipcMain, invoke, on, off } from './ipc.js';
export type { IpcErrorCode, IpcProcessOptions, IpcProcessResponse } from './ipc.js';
export {
  createContractInvoker,
  createLegacyInvokeAdapter,
  createSchema,
  defineCommands,
  isIpcContractValidationError,
  registerContractHandlers,
  resolveContractChannel,
  IpcContractValidationError,
  IpcSchema,
} from './ipc-contract.js';
export type {
  ContractHandlers,
  InferCommandRequest,
  InferCommandResponse,
  InferSchemaValue,
  IpcCommandDefinition,
  IpcCommandMap,
  IpcInvokeFn,
  IpcRegistrar,
  IpcSchema as IpcSchemaType,
  TypedIpcInvoker,
} from './ipc-contract.js';
export { Tray } from './tray.js';
export { Menu, MenuItem } from './menu.js';
export { dialog } from './dialog.js';
export { clipboard } from './clipboard.js';
export { Notification } from './notification.js';
export { fs } from './fs.js';
export { shell } from './shell.js';
export { globalShortcut } from './globalShortcut.js';
export { autoUpdater } from './updater.js';
export { defineConfig } from './types.js';
export type {
  VoltConfig,
  WindowOptions,
  Permission,
  BuildConfig,
  PackageConfig,
  SigningConfig,
  MacOSSigningConfig,
  WindowsSigningConfig,
  AzureTrustedSigningConfig,
  DigiCertKeyLockerConfig,
  UpdaterConfig,
  UpdaterTelemetryConfig,
  RuntimeConfig,
} from './types.js';
export type { TrayOptions, TrayMenuItem } from './tray.js';
export type { MenuItemOptions, MenuItemRole } from './menu.js';
export type {
  OpenDialogOptions,
  OpenDialogResult,
  SaveDialogOptions,
  SaveDialogResult,
  MessageBoxOptions,
  MessageBoxResult,
  FileFilter,
} from './dialog.js';
export type { ClipboardImage } from './clipboard.js';
export type { NotificationOptions } from './notification.js';
export type { FileInfo } from './fs.js';
export type { UpdateInfo } from './updater.js';
