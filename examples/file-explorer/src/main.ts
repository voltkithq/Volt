/**
 * File Explorer — renderer
 *
 * Calls backend IPC handlers to pick a folder, list contents,
 * and watch for changes. No direct filesystem access from here —
 * everything goes through the backend via IPC.
 */

const volt = (window as any).__volt__;

const openBtn = document.getElementById('open-btn')!;
const fileList = document.getElementById('file-list')!;
const watchLog = document.getElementById('watch-log')!;

openBtn.addEventListener('click', async () => {
  const result = await volt.invoke('folder:pick');
  if (!result.ok) return;

  openBtn.textContent = `Watching: ${result.path}`;
  await listFiles('');
  await volt.invoke('watch:start');
});

async function listFiles(path: string) {
  const items = await volt.invoke('folder:list', { path });
  fileList.innerHTML = items
    .map(
      (item: any) =>
        `<div class="entry ${item.isDir ? 'dir' : 'file'}">
          ${item.isDir ? '📁' : '📄'} ${item.name}
          <span class="meta">${item.isDir ? '' : formatSize(item.size)}</span>
        </div>`,
    )
    .join('');
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// Listen for file change events from the watcher
volt.on('watch:events', (events: any[]) => {
  for (const event of events) {
    const line = document.createElement('div');
    line.className = `event ${event.kind}`;
    line.textContent = `[${event.kind}] ${event.path}`;
    watchLog.prepend(line);
  }
  // Keep only last 50 entries
  while (watchLog.children.length > 50) {
    watchLog.removeChild(watchLog.lastChild!);
  }
});
