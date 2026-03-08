export interface StatusResponse {
  os: {
    platform: string;
    arch: string;
  };
  generatedUuid: string;
  clipboard: {
    read: string;
    hasText: boolean;
  };
  runtime: {
    windowCount: number;
    dbRows: number;
    nativeReady: boolean;
    shortcut: string;
    secureStorageDemoKey: string;
    secureStorageHasDemoKey: boolean;
  };
}

export interface DbRecord {
  id: string;
  message: string;
  createdAt: number;
}

export interface SecureStorageSetResponse {
  ok: boolean;
  key: string;
  has: boolean;
}

export interface SecureStorageGetResponse {
  key: string;
  value: string | null;
  has: boolean;
}

export interface SecureStorageHasResponse {
  key: string;
  has: boolean;
}

export type WindowAction = 'window:minimize' | 'window:maximize' | 'window:restore';
