export class HeartbeatMonitor {
  private timer: ReturnType<typeof setInterval> | null = null;
  private missedHeartbeats = 0;
  private awaitingAck = false;

  constructor(
    private readonly intervalMs: number,
    private readonly timeoutMs: number,
    private readonly sendHeartbeat: () => void,
    private readonly onUnresponsive: () => void,
  ) {}

  start(): void {
    if (this.timer) return;
    this.missedHeartbeats = 0;
    this.awaitingAck = false;
    this.timer = setInterval(() => this.tick(), this.intervalMs);
  }

  stop(): void {
    if (!this.timer) return;
    clearInterval(this.timer);
    this.timer = null;
  }

  acknowledge(): void {
    this.awaitingAck = false;
    this.missedHeartbeats = 0;
  }

  private tick(): void {
    void this.timeoutMs;

    if (this.awaitingAck) {
      this.missedHeartbeats++;
      if (this.missedHeartbeats >= 2) {
        this.onUnresponsive();
        return;
      }
    }

    this.awaitingAck = true;
    this.sendHeartbeat();
  }
}
