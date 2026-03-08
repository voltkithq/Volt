import { describe, expect, expectTypeOf, it, vi } from 'vitest';
import {
  createContractInvoker,
  createLegacyInvokeAdapter,
  createSchema,
  defineCommands,
  IpcSchema,
  IpcContractValidationError,
  isIpcContractValidationError,
  registerContractHandlers,
  resolveContractChannel,
} from '../ipc-contract.js';
import { ipcMain } from '../ipc.js';

interface ComputeArgs {
  a: number;
  b: number;
}

interface ComputeResult {
  sum: number;
}

const isComputeArgs = (value: unknown): value is ComputeArgs => {
  if (!value || typeof value !== 'object') {
    return false;
  }
  const record = value as Record<string, unknown>;
  return typeof record['a'] === 'number' && typeof record['b'] === 'number';
};

const isComputeResult = (value: unknown): value is ComputeResult => {
  if (!value || typeof value !== 'object') {
    return false;
  }
  const record = value as Record<string, unknown>;
  return typeof record['sum'] === 'number';
};

describe('ipc-contract helpers', () => {
  it('registers typed handlers with legacy aliases and validates payloads', async () => {
    const commands = defineCommands({
      'demo.compute': {
        request: createSchema<ComputeArgs>('ComputeArgs', isComputeArgs),
        response: createSchema<ComputeResult>('ComputeResult', isComputeResult),
        aliases: ['compute'],
      },
    });

    registerContractHandlers(ipcMain, commands, {
      'demo.compute': ({ a, b }) => ({ sum: a + b }),
    });

    const ok = await ipcMain.processRequest('req-1', 'compute', { a: 2, b: 5 });
    expect(ok.error).toBeUndefined();
    expect(ok.result).toEqual({ sum: 7 });

    const invalidArgs = await ipcMain.processRequest('req-2', 'compute', { a: 2, b: 'x' });
    expect(invalidArgs.errorCode).toBe('IPC_HANDLER_ERROR');
    expect(invalidArgs.error).toContain('request validation failed');

    ipcMain.removeHandler('demo.compute');
    ipcMain.removeHandler('compute');
  });

  it('throws when a handler emits a response that violates the contract', async () => {
    const commands = defineCommands({
      'demo.bad': {
        request: IpcSchema.null('NullPayload'),
        response: createSchema<ComputeResult>('ComputeResult', isComputeResult),
      },
    });

    registerContractHandlers(ipcMain, commands, {
      'demo.bad': () => ({ value: 42 }) as unknown as ComputeResult,
    });

    const invalidResult = await ipcMain.processRequest('req-3', 'demo.bad', null);
    expect(invalidResult.errorCode).toBe('IPC_HANDLER_ERROR');
    expect(invalidResult.error).toContain('response validation failed');

    ipcMain.removeHandler('demo.bad');
  });

  it('supports built-in schema helpers for concise contracts', () => {
    const payloadSchema = IpcSchema.object({
      a: IpcSchema.number(),
      b: IpcSchema.number(),
      meta: IpcSchema.optional(
        IpcSchema.object({
          label: IpcSchema.string(),
        }),
      ),
    }, 'Payload');
    const responseSchema = IpcSchema.object({
      ok: IpcSchema.boolean(),
      values: IpcSchema.array(IpcSchema.number()),
      code: IpcSchema.literal('done'),
    }, 'Response');

    expect(payloadSchema.parse({ a: 1, b: 2 })).toEqual({ a: 1, b: 2, meta: undefined });
    expect(responseSchema.parse({ ok: true, values: [1, 2], code: 'done' })).toEqual({
      ok: true,
      values: [1, 2],
      code: 'done',
    });
    expect(() => payloadSchema.parse({ a: 1, b: 'x' })).toThrow(/expected number/);
  });

  it('fails early when command aliases conflict', () => {
    expect(() =>
      defineCommands({
        'demo.alpha': {
          aliases: ['conflict'],
        },
        'demo.beta': {
          aliases: ['conflict'],
        },
      }),
    ).toThrow('IPC contract alias conflict');
  });

  it('creates a typed renderer invoker and resolves aliases', async () => {
    const commands = defineCommands({
      'demo.compute': {
        request: createSchema<ComputeArgs>('ComputeArgs', isComputeArgs),
        response: createSchema<ComputeResult>('ComputeResult', isComputeResult),
        aliases: ['compute'],
      },
    });

    const invokeFn = vi.fn(async (channel: string, args: unknown) => {
      if (channel !== 'demo.compute') {
        throw new Error(`unexpected channel: ${channel}`);
      }
      const payload = args as ComputeArgs;
      return { sum: payload.a + payload.b };
    });

    const invoker = createContractInvoker(commands, invokeFn);

    const result = await invoker.invoke('demo.compute', { a: 1, b: 9 });
    expect(result).toEqual({ sum: 10 });

    expect(invoker.resolveChannel('compute')).toBe('demo.compute');
    const legacyResult = await invoker.invokeLegacy('compute', { a: 3, b: 7 });
    expect(legacyResult).toEqual({ sum: 10 });

    expect(invokeFn).toHaveBeenNthCalledWith(1, 'demo.compute', { a: 1, b: 9 });
    expect(invokeFn).toHaveBeenNthCalledWith(2, 'demo.compute', { a: 3, b: 7 });
  });

  it('creates a legacy invoke adapter for untyped callers', async () => {
    const commands = defineCommands({
      'demo.ping': {
        aliases: ['ping'],
      },
    });

    const invokeFn = vi.fn(async () => ({ ok: true }));
    const invokeLegacy = createLegacyInvokeAdapter(commands, invokeFn);

    const response = await invokeLegacy('ping', null);
    expect(response).toEqual({ ok: true });
    expect(invokeFn).toHaveBeenCalledWith('demo.ping', null);
  });

  it('exposes useful type inference for contract invocations', () => {
    const commands = defineCommands({
      'demo.compute': {
        request: createSchema<ComputeArgs>('ComputeArgs', isComputeArgs),
        response: createSchema<ComputeResult>('ComputeResult', isComputeResult),
      },
    });

    const invoker = createContractInvoker(commands, async () => ({ sum: 0 }));
    const promise = invoker.invoke('demo.compute', { a: 4, b: 6 });
    expectTypeOf(promise).toEqualTypeOf<Promise<ComputeResult>>();

    const resolved = resolveContractChannel(commands, 'demo.compute');
    expect(resolved).toBe('demo.compute');
  });

  it('identifies contract validation errors', () => {
    const error = new IpcContractValidationError('demo.compute', 'request', 'bad request', {
      reason: 'schema mismatch',
    });

    expect(isIpcContractValidationError(error)).toBe(true);
    expect(error.code).toBe('IPC_CONTRACT_VALIDATION_ERROR');
  });
});
