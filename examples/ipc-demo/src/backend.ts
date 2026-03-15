import { ipcMain } from 'volt:ipc';
import { registerContractHandlers } from 'voltkit/ipc-contract';
import * as voltEvents from 'volt:events';
import * as voltWindow from 'volt:window';

import { registerDemoHandlers } from './backend/handlers.js';
import { registerNativeEventBridge } from './backend/native.js';
import {
  ipcCommands,
  type ComputeRequest,
  type EchoRequest,
  type PingResult,
} from './ipc-contract.js';

registerNativeEventBridge();

registerContractHandlers(ipcMain, ipcCommands, {
  'demo.ping': (_payload): PingResult => ({ pong: Date.now() }),
  'demo.echo': (payload: EchoRequest) => payload,
  'demo.compute': (payload: ComputeRequest) => ({
    sum: payload.a + payload.b,
    product: payload.a * payload.b,
  }),
});

registerDemoHandlers({ ipcMain, voltEvents, voltWindow });
