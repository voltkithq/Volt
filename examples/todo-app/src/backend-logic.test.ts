import { describe, expect, it } from 'vitest';
import { toTodoRecord } from './backend-logic.js';

describe('todo-app backend logic', () => {
  it('maps database rows into TodoRecord objects', () => {
    const todo = toTodoRecord({
      id: 'todo-1',
      text: 'Ship feature',
      completed: 1,
      created_at: 123,
    });

    expect(todo).toEqual({
      id: 'todo-1',
      text: 'Ship feature',
      completed: true,
      createdAt: 123,
    });
  });

  it('normalizes invalid/missing row fields safely', () => {
    const todo = toTodoRecord({
      completed: 'not-a-number',
      created_at: 'bad',
    });

    expect(todo).toEqual({
      id: '',
      text: '',
      completed: false,
      createdAt: 0,
    });
  });
});
