function ratio(boaValue, nodeValue) {
  if (typeof boaValue !== 'number' || typeof nodeValue !== 'number' || nodeValue <= 0) {
    return null;
  }
  return Number((boaValue / nodeValue).toFixed(2));
}

function metric(caseSummary, key) {
  return caseSummary?.metrics?.[key];
}

function buildRatios(nodeSummary, boaSummary) {
  return {
    analyticsStudio: {
      backendDurationMs: ratio(
        metric(boaSummary.analyticsStudio, 'backendDurationMs'),
        metric(nodeSummary.analyticsStudio, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(boaSummary.analyticsStudio, 'roundTripMs'),
        metric(nodeSummary.analyticsStudio, 'roundTripMs'),
      ),
    },
    syncStorm: {
      backendDurationMs: ratio(
        metric(boaSummary.syncStorm, 'backendDurationMs'),
        metric(nodeSummary.syncStorm, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(boaSummary.syncStorm, 'roundTripMs'),
        metric(nodeSummary.syncStorm, 'roundTripMs'),
      ),
    },
    workflowLab: {
      backendDurationMs: ratio(
        metric(boaSummary.workflowLab, 'backendDurationMs'),
        metric(nodeSummary.workflowLab, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(boaSummary.workflowLab, 'roundTripMs'),
        metric(nodeSummary.workflowLab, 'roundTripMs'),
      ),
    },
  };
}

function buildSpeedups(jsSummary, nativeSummary) {
  return {
    analyticsStudio: {
      backendDurationMs: ratio(
        metric(jsSummary.analyticsStudio, 'backendDurationMs'),
        metric(nativeSummary.analyticsStudio, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(jsSummary.analyticsStudio, 'roundTripMs'),
        metric(nativeSummary.analyticsStudio, 'roundTripMs'),
      ),
    },
    syncStorm: {
      backendDurationMs: ratio(
        metric(jsSummary.syncStorm, 'backendDurationMs'),
        metric(nativeSummary.syncStorm, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(jsSummary.syncStorm, 'roundTripMs'),
        metric(nativeSummary.syncStorm, 'roundTripMs'),
      ),
    },
    workflowLab: {
      backendDurationMs: ratio(
        metric(jsSummary.workflowLab, 'backendDurationMs'),
        metric(nativeSummary.workflowLab, 'backendDurationMs'),
      ),
      roundTripMs: ratio(
        metric(jsSummary.workflowLab, 'roundTripMs'),
        metric(nativeSummary.workflowLab, 'roundTripMs'),
      ),
    },
  };
}

function buildVariantSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary) {
  return {
    node: nodeSummary,
    boaJs: boaJsSummary,
    boaNative: boaNativeSummary,
    directNative: directNativeSummary,
    ratios: {
      boaJsVsNode: buildRatios(nodeSummary, boaJsSummary),
      boaNativeVsNode: buildRatios(nodeSummary, boaNativeSummary),
      directNativeVsNode: buildRatios(nodeSummary, directNativeSummary),
      forwardedNativeSpeedup: buildSpeedups(boaJsSummary, boaNativeSummary),
      directNativeSpeedup: buildSpeedups(boaJsSummary, directNativeSummary),
      directVsForwardedSpeedup: buildSpeedups(boaNativeSummary, directNativeSummary),
    },
  };
}

function buildSingleSummary(
  isReleaseMode,
  nodeSummary,
  boaJsSummary,
  boaNativeSummary,
  directNativeSummary,
) {
  return {
    generatedAt: new Date().toISOString(),
    platform: process.platform,
    mode: 'headless-backend-runtime',
    boaProfile: isReleaseMode ? 'release' : 'test',
    ...buildVariantSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary),
  };
}

export { buildSingleSummary, buildVariantSummary };
