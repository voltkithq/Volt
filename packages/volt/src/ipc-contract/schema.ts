import type { InferSchemaValue, IpcSchema as IpcSchemaType } from './types.js';

type IpcObjectShape = Record<string, IpcSchemaType<unknown>>;

export function createSchema<T>(
  name: string,
  guard: (value: unknown) => value is T,
): IpcSchemaType<T> {
  return {
    name,
    parse(value: unknown): T {
      if (guard(value)) {
        return value;
      }
      throw new Error(`expected ${name}`);
    },
  };
}

export const IpcSchema = {
  custom<T>(name: string, guard: (value: unknown) => value is T): IpcSchemaType<T> {
    return createSchema(name, guard);
  },

  unknown(name = 'unknown'): IpcSchemaType<unknown> {
    return {
      name,
      parse(value: unknown): unknown {
        return value;
      },
    };
  },

  null(name = 'null'): IpcSchemaType<null> {
    return createSchema<null>(name, (value): value is null => value === null);
  },

  string(name = 'string'): IpcSchemaType<string> {
    return createSchema<string>(name, (value): value is string => typeof value === 'string');
  },

  number(name = 'number'): IpcSchemaType<number> {
    return createSchema<number>(name, (value): value is number => typeof value === 'number');
  },

  boolean(name = 'boolean'): IpcSchemaType<boolean> {
    return createSchema<boolean>(name, (value): value is boolean => typeof value === 'boolean');
  },

  literal<TLiteral extends string | number | boolean | null>(
    expected: TLiteral,
    name = `literal(${String(expected)})`,
  ): IpcSchemaType<TLiteral> {
    return createSchema<TLiteral>(name, (value): value is TLiteral => value === expected);
  },

  optional<T>(
    schema: IpcSchemaType<T>,
    name = `optional(${schema.name ?? 'value'})`,
  ): IpcSchemaType<T | undefined> {
    return {
      name,
      parse(value: unknown): T | undefined {
        if (value === undefined) {
          return undefined;
        }
        return schema.parse(value);
      },
    };
  },

  array<TItem>(
    itemSchema: IpcSchemaType<TItem>,
    name = `array(${itemSchema.name ?? 'item'})`,
  ): IpcSchemaType<TItem[]> {
    return {
      name,
      parse(value: unknown): TItem[] {
        if (!Array.isArray(value)) {
          throw new Error(`expected ${name}`);
        }
        return value.map((item) => itemSchema.parse(item));
      },
    };
  },

  object<TShape extends IpcObjectShape>(
    shape: TShape,
    name = 'object',
  ): IpcSchemaType<{ [TKey in keyof TShape]: InferSchemaValue<TShape[TKey]> }> {
    return {
      name,
      parse(value: unknown): { [TKey in keyof TShape]: InferSchemaValue<TShape[TKey]> } {
        if (!value || typeof value !== 'object' || Array.isArray(value)) {
          throw new Error(`expected ${name}`);
        }

        const record = value as Record<string, unknown>;
        const parsed = {} as { [TKey in keyof TShape]: InferSchemaValue<TShape[TKey]> };
        for (const key of Object.keys(shape) as Array<keyof TShape>) {
          parsed[key] = shape[key].parse(record[key as string]) as InferSchemaValue<TShape[typeof key]>;
        }
        return parsed;
      },
    };
  },
} as const;
