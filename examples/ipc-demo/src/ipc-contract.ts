import {
  IpcSchema,
  defineCommands,
  type InferCommandRequest,
  type InferCommandResponse,
} from 'voltkit/renderer';

export interface PingResponse {
  pong: number;
}

export interface EchoPayload {
  message: string;
  sentAt: string;
}

export interface ComputeArgs {
  a: number;
  b: number;
}

export interface ComputeResponse {
  sum: number;
  product: number;
}

const pingResponseSchema = IpcSchema.object({
  pong: IpcSchema.number(),
}, 'PingResponse');

const echoPayloadSchema = IpcSchema.object({
  message: IpcSchema.string(),
  sentAt: IpcSchema.string(),
}, 'EchoPayload');

const computeArgsSchema = IpcSchema.object({
  a: IpcSchema.number(),
  b: IpcSchema.number(),
}, 'ComputeArgs');

const computeResponseSchema = IpcSchema.object({
  sum: IpcSchema.number(),
  product: IpcSchema.number(),
}, 'ComputeResponse');

export const ipcCommands = defineCommands({
  'demo.ping': {
    request: IpcSchema.null('NullPayload'),
    response: pingResponseSchema,
    aliases: ['ping'],
  },
  'demo.echo': {
    request: echoPayloadSchema,
    response: echoPayloadSchema,
    aliases: ['echo'],
  },
  'demo.compute': {
    request: computeArgsSchema,
    response: computeResponseSchema,
    aliases: ['compute'],
  },
});

export type IpcCommands = typeof ipcCommands;

export type PingRequest = InferCommandRequest<IpcCommands['demo.ping']>;
export type PingResult = InferCommandResponse<IpcCommands['demo.ping']>;
export type EchoRequest = InferCommandRequest<IpcCommands['demo.echo']>;
export type EchoResult = InferCommandResponse<IpcCommands['demo.echo']>;
export type ComputeRequest = InferCommandRequest<IpcCommands['demo.compute']>;
export type ComputeResult = InferCommandResponse<IpcCommands['demo.compute']>;
