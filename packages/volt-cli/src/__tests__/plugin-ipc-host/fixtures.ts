import { ChildProcess, spawn } from 'node:child_process';

const ECHO_SCRIPT = `
const { Buffer } = require('buffer');

function writeFrame(msg) {
  const body = Buffer.from(JSON.stringify(msg) + '\\n', 'utf-8');
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  process.stdout.write(Buffer.concat([header, body]));
}

writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });

let buf = Buffer.alloc(0);
process.stdin.on('data', (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  while (buf.length >= 4) {
    const len = buf.readUInt32LE(0);
    if (buf.length < 4 + len) break;
    const raw = buf.subarray(4, 4 + len).toString('utf-8');
    const msg = JSON.parse(raw.endsWith('\\n') ? raw.slice(0, -1) : raw);
    buf = buf.subarray(4 + len);

    if (msg.type === 'signal' && msg.method === 'heartbeat') {
      writeFrame({ type: 'signal', id: msg.id, method: 'heartbeat-ack', payload: null, error: null });
    } else if (msg.type === 'signal' && msg.method === 'deactivate') {
      process.exit(0);
    } else if (msg.type === 'request') {
      writeFrame({ type: 'response', id: msg.id, method: msg.method, payload: msg.payload, error: null });
    }
  }
});

process.stdin.on('end', () => process.exit(0));
`;

const SLOW_ECHO_SCRIPT = `
const { Buffer } = require('buffer');

function writeFrame(msg) {
  const body = Buffer.from(JSON.stringify(msg) + '\\n', 'utf-8');
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  process.stdout.write(Buffer.concat([header, body]));
}

writeFrame({ type: 'signal', id: 'init', method: 'ready', payload: null, error: null });

let buf = Buffer.alloc(0);
process.stdin.on('data', (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  while (buf.length >= 4) {
    const len = buf.readUInt32LE(0);
    if (buf.length < 4 + len) break;
    const raw = buf.subarray(4, 4 + len).toString('utf-8');
    const msg = JSON.parse(raw.endsWith('\\n') ? raw.slice(0, -1) : raw);
    buf = buf.subarray(4 + len);

    if (msg.type === 'signal' && msg.method === 'heartbeat') {
      writeFrame({ type: 'signal', id: msg.id, method: 'heartbeat-ack', payload: null, error: null });
    } else if (msg.type === 'signal' && msg.method === 'deactivate') {
      process.exit(0);
    } else if (msg.type === 'request') {
      setTimeout(() => {
        writeFrame({ type: 'response', id: msg.id, method: msg.method, payload: msg.payload, error: null });
      }, 500);
    }
  }
});

process.stdin.on('end', () => process.exit(0));
`;

export function spawnEcho(): ChildProcess {
  return spawn(process.execPath, ['-e', ECHO_SCRIPT], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
}

export function spawnSlowEcho(): ChildProcess {
  return spawn(process.execPath, ['-e', SLOW_ECHO_SCRIPT], {
    stdio: ['pipe', 'pipe', 'pipe'],
  });
}

export async function killProcess(proc: ChildProcess | undefined): Promise<void> {
  if (!proc || proc.exitCode !== null) return;
  proc.kill('SIGKILL');
  await new Promise<void>((resolve) => proc.on('exit', () => resolve()));
}
