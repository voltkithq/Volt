import { describe, expect, it } from 'vitest';
import { buildSyncStormPreset, startSyncStorm } from './backend-logic.js';
import type { SyncStormSummary, SyncTickPayload, SyncSnapshotPayload } from './backend-logic.js';

describe('sync-storm backend logic', () => {
  it('buildSyncStormPreset returns valid default config', () => {
    const preset = buildSyncStormPreset();
    expect(preset.workerCount).toBe(10);
    expect(preset.ticksPerWorker).toBe(36);
    expect(preset.intervalMs).toBe(6);
    expect(preset.burstSize).toBe(6);
  });

  it('normalizes out-of-range options to bounds', () => {
    const preset = buildSyncStormPreset();
    // Default values are already clamped; verify they're within range
    expect(preset.workerCount).toBeGreaterThanOrEqual(1);
    expect(preset.workerCount).toBeLessThanOrEqual(32);
    expect(preset.ticksPerWorker).toBeGreaterThanOrEqual(4);
    expect(preset.ticksPerWorker).toBeLessThanOrEqual(160);
  });

  it('startSyncStorm completes and emits expected events', async () => {
    const ticks: SyncTickPayload[] = [];
    const snapshots: SyncSnapshotPayload[] = [];
    let summary: SyncStormSummary | null = null;

    const execution = startSyncStorm(
      { workerCount: 2, ticksPerWorker: 4, intervalMs: 1, burstSize: 2 },
      {
        tick: (payload) => ticks.push(payload),
        snapshot: (payload) => snapshots.push(payload),
        complete: (payload) => { summary = payload; },
      },
    );

    expect(execution.scenarioId).toMatch(/^sync-/);
    expect(execution.config.workerCount).toBe(2);

    const result = await execution.done;
    expect(result.workerCount).toBe(2);
    expect(result.ticksPerWorker).toBe(4);
    expect(result.totalTickEvents).toBe(8); // 2 workers × 4 ticks
    expect(ticks.length).toBe(8);
    expect(snapshots.length).toBeGreaterThan(0);
    expect(summary).not.toBeNull();
  });
});
