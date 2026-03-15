function logSummary(summary, isReleaseMode, isSweepMode, summaryPath) {
  console.log('[bench] Wrote benchmark summary:', summaryPath);
  console.log(`[bench] Boa profile: ${isReleaseMode ? 'release' : 'test'}`);

  if (isSweepMode) {
    for (const profile of summary.profiles) {
      console.log(
        `[bench] ${profile.id} analytics js ratio=${profile.ratios.boaJsVsNode.analyticsStudio.backendDurationMs} forwarded ratio=${profile.ratios.boaNativeVsNode.analyticsStudio.backendDurationMs} direct ratio=${profile.ratios.directNativeVsNode.analyticsStudio.backendDurationMs} direct speedup=${profile.ratios.directNativeSpeedup.analyticsStudio.backendDurationMs}`,
      );
      console.log(
        `[bench] ${profile.id} sync js ratio=${profile.ratios.boaJsVsNode.syncStorm.backendDurationMs} forwarded ratio=${profile.ratios.boaNativeVsNode.syncStorm.backendDurationMs} direct ratio=${profile.ratios.directNativeVsNode.syncStorm.backendDurationMs} direct speedup=${profile.ratios.directNativeSpeedup.syncStorm.backendDurationMs}`,
      );
      console.log(
        `[bench] ${profile.id} workflow js ratio=${profile.ratios.boaJsVsNode.workflowLab.backendDurationMs} forwarded ratio=${profile.ratios.boaNativeVsNode.workflowLab.backendDurationMs} direct ratio=${profile.ratios.directNativeVsNode.workflowLab.backendDurationMs} direct speedup=${profile.ratios.directNativeSpeedup.workflowLab.backendDurationMs}`,
      );
    }
    return;
  }

  console.log(
    `[bench] analytics js ratio=${summary.ratios.boaJsVsNode.analyticsStudio.backendDurationMs} forwarded ratio=${summary.ratios.boaNativeVsNode.analyticsStudio.backendDurationMs} direct ratio=${summary.ratios.directNativeVsNode.analyticsStudio.backendDurationMs} direct speedup=${summary.ratios.directNativeSpeedup.analyticsStudio.backendDurationMs}`,
  );
  console.log(
    `[bench] sync js ratio=${summary.ratios.boaJsVsNode.syncStorm.backendDurationMs} forwarded ratio=${summary.ratios.boaNativeVsNode.syncStorm.backendDurationMs} direct ratio=${summary.ratios.directNativeVsNode.syncStorm.backendDurationMs} direct speedup=${summary.ratios.directNativeSpeedup.syncStorm.backendDurationMs}`,
  );
  console.log(
    `[bench] workflow js ratio=${summary.ratios.boaJsVsNode.workflowLab.backendDurationMs} forwarded ratio=${summary.ratios.boaNativeVsNode.workflowLab.backendDurationMs} direct ratio=${summary.ratios.directNativeVsNode.workflowLab.backendDurationMs} direct speedup=${summary.ratios.directNativeSpeedup.workflowLab.backendDurationMs}`,
  );
}

export { logSummary };
