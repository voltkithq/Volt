export interface TodoRecord {
  id: string;
  text: string;
  completed: boolean;
  createdAt: number;
}

export function toTodoRecord(row: unknown): TodoRecord {
  const value = (row ?? {}) as Record<string, unknown>;
  const id = typeof value.id === 'string' ? value.id : '';
  const text = typeof value.text === 'string' ? value.text : '';
  const completedRaw = value.completed;
  const completedNumber = typeof completedRaw === 'number' ? completedRaw : Number(completedRaw ?? 0);
  const createdAtRaw = value.created_at;
  const createdAt = typeof createdAtRaw === 'number' ? createdAtRaw : Number(createdAtRaw ?? 0);

  return {
    id,
    text,
    completed: completedNumber === 1,
    createdAt: Number.isFinite(createdAt) ? createdAt : 0,
  };
}
