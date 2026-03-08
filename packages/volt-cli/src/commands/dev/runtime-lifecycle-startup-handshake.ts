export interface NativeHostStartupHandshake {
  readonly ready: Promise<void>;
  isReadySettled(): boolean;
  resolveReady(): void;
  rejectReady(reason: unknown): void;
}

export function createNativeHostStartupHandshake(): NativeHostStartupHandshake {
  let readySettled = false;
  let readyResolve!: () => void;
  let readyReject!: (reason?: unknown) => void;

  const ready = new Promise<void>((resolve, reject) => {
    readyResolve = resolve;
    readyReject = reject;
  });

  return {
    ready,
    isReadySettled(): boolean {
      return readySettled;
    },
    resolveReady(): void {
      if (readySettled) {
        return;
      }
      readySettled = true;
      readyResolve();
    },
    rejectReady(reason: unknown): void {
      if (readySettled) {
        return;
      }
      readySettled = true;
      readyReject(reason);
    },
  };
}
