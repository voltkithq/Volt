import { writeFileSync } from 'node:fs';

const scenario = process.env.VOLT_TEST_HOST_SCENARIO ?? 'ready';
const pidFile = process.env.VOLT_TEST_HOST_PID_FILE;
const PROTOCOL_VERSION = 1;

if (pidFile) {
  writeFileSync(pidFile, String(process.pid), 'utf8');
}

function send(message) {
  if (typeof process.send === 'function') {
    process.send({
      protocolVersion: PROTOCOL_VERSION,
      ...message,
    });
  }
}

function keepAlive() {
  setInterval(() => {}, 1000);
}

process.on('message', (raw) => {
  if (!raw || typeof raw !== 'object') {
    return;
  }

  const message = raw;
  if (message.protocolVersion !== PROTOCOL_VERSION) {
    send({
      type: 'runtime-error',
      message: `unsupported protocol version: ${String(message.protocolVersion)}`,
    });
    process.exit(1);
    return;
  }

  if (message.type === 'start') {
    switch (scenario) {
      case 'ready':
        send({ type: 'starting' });
        send({ type: 'ready' });
        break;
      case 'runtime-error-exit':
        send({ type: 'starting' });
        send({ type: 'runtime-error', message: 'fake runtime failure' });
        process.exit(1);
        break;
      case 'runtime-error-hang':
        send({ type: 'starting' });
        send({ type: 'runtime-error', message: 'fake runtime failure' });
        keepAlive();
        break;
      case 'native-unavailable':
        send({ type: 'starting' });
        send({ type: 'native-unavailable', message: 'fake native unavailable' });
        keepAlive();
        break;
      case 'early-exit':
        process.exit(23);
        break;
      case 'silent-hang':
        send({ type: 'starting' });
        keepAlive();
        break;
      case 'wrong-version-ready':
        if (typeof process.send === 'function') {
          process.send({ type: 'starting', protocolVersion: 999 });
          process.send({ type: 'ready', protocolVersion: 999 });
        }
        keepAlive();
        break;
      case 'no-pong':
        send({ type: 'starting' });
        send({ type: 'ready' });
        keepAlive();
        break;
      case 'delayed-pong':
        send({ type: 'starting' });
        send({ type: 'ready' });
        keepAlive();
        break;
      case 'crash-after-ready':
        send({ type: 'starting' });
        send({ type: 'ready' });
        setTimeout(() => process.exit(19), 50).unref();
        break;
      case 'disconnect-after-ready':
        send({ type: 'starting' });
        send({ type: 'ready' });
        if (typeof process.disconnect === 'function') {
          process.disconnect();
        }
        keepAlive();
        break;
      case 'window-closed-then-quit':
        send({ type: 'starting' });
        send({ type: 'ready' });
        setTimeout(() => {
          send({
            type: 'event',
            eventJson: JSON.stringify({
              type: 'window-closed',
              windowId: 'WindowId(5)',
              jsWindowId: 'window-5',
            }),
          });
          send({
            type: 'event',
            eventJson: JSON.stringify({
              type: 'quit',
            }),
          });
        }, 50).unref();
        keepAlive();
        break;
      default:
        send({ type: 'runtime-error', message: `unknown scenario: ${scenario}` });
        process.exit(1);
        break;
    }
    return;
  }

  if (message.type === 'ping') {
    if (scenario === 'no-pong') {
      return;
    }
    if (scenario === 'delayed-pong') {
      const delayMs = Number.parseInt(process.env.VOLT_TEST_HOST_PONG_DELAY_MS ?? '400', 10);
      setTimeout(() => {
        send({
          type: 'pong',
          pingId: message.pingId,
        });
      }, Number.isFinite(delayMs) ? delayMs : 400).unref();
      return;
    }
    send({
      type: 'pong',
      pingId: message.pingId,
    });
    return;
  }

  if (message.type === 'shutdown') {
    send({ type: 'stopping' });
    send({ type: 'stopped', code: 0 });
    process.exit(0);
  }
});
