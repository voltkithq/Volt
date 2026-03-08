export {};

interface Todo {
  id: string;
  text: string;
  completed: boolean;
  createdAt: number;
}

interface VoltBridge {
  invoke(method: string, args?: unknown): Promise<unknown>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

const input = document.getElementById('todo-input') as HTMLInputElement;
const addBtn = document.getElementById('add-btn') as HTMLButtonElement;
const todoList = document.getElementById('todo-list') as HTMLUListElement;
const stats = document.getElementById('stats') as HTMLParagraphElement;

function getBridge(): VoltBridge | null {
  if (window.__volt__?.invoke) {
    return window.__volt__;
  }
  return null;
}

function updateRuntimeAvailabilityUi(): boolean {
  const hasBridge = getBridge() !== null;
  if (!hasBridge) {
    input.disabled = true;
    addBtn.disabled = true;
    stats.textContent = 'Volt runtime is not attached. Run this app with `npx volt dev` or `npx volt build`.';
  } else {
    input.disabled = false;
    addBtn.disabled = false;
  }
  return hasBridge;
}

async function invoke<T>(method: string, args?: unknown): Promise<T> {
  const bridge = getBridge();
  if (!bridge) {
    throw new Error('window.__volt__.invoke is unavailable');
  }
  return bridge.invoke(method, args) as Promise<T>;
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function showError(error: unknown): void {
  stats.textContent = `Error: ${toErrorMessage(error)}`;
}

async function render(): Promise<void> {
  if (!updateRuntimeAvailabilityUi()) {
    return;
  }

  try {
    const allTodos = await invoke<Todo[]>('get-todos');
    todoList.innerHTML = '';

    for (const todo of allTodos) {
      const li = document.createElement('li');
      li.className = `todo-item${todo.completed ? ' completed' : ''}`;

      const text = document.createElement('span');
      text.className = 'todo-text';
      text.textContent = todo.text;
      text.addEventListener('click', () => {
        void toggleTodo(todo.id);
      });

      const deleteBtn = document.createElement('button');
      deleteBtn.className = 'delete-btn';
      deleteBtn.textContent = 'Delete';
      deleteBtn.addEventListener('click', () => {
        void deleteTodo(todo.id);
      });

      li.appendChild(text);
      li.appendChild(deleteBtn);
      todoList.appendChild(li);
    }

    const total = allTodos.length;
    const completed = allTodos.filter((todo) => todo.completed).length;
    stats.textContent =
      total === 0
        ? 'No todos yet. Add one above!'
        : `${completed}/${total} completed`;
  } catch (error) {
    showError(error);
  }
}

async function addTodo(): Promise<void> {
  if (!updateRuntimeAvailabilityUi()) {
    return;
  }

  const text = input.value.trim();
  if (!text) {
    return;
  }

  try {
    await invoke<Todo>('add-todo', { text });
    input.value = '';
    await render();
  } catch (error) {
    showError(error);
  }
}

async function toggleTodo(id: string): Promise<void> {
  if (!updateRuntimeAvailabilityUi()) {
    return;
  }

  try {
    await invoke<Todo>('toggle-todo', { id });
    await render();
  } catch (error) {
    showError(error);
  }
}

async function deleteTodo(id: string): Promise<void> {
  if (!updateRuntimeAvailabilityUi()) {
    return;
  }

  try {
    await invoke<{ success: boolean }>('delete-todo', { id });
    await render();
  } catch (error) {
    showError(error);
  }
}

addBtn.addEventListener('click', () => {
  void addTodo();
});

input.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    void addTodo();
  }
});

void render();
