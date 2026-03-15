import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import {
  backendBuildModulePath,
  benchmarkProjects,
  benchmarkSweepProfiles,
  isReleaseMode,
  nodeBenchmarkBundlePath,
  nodeBenchmarkScript,
  repoRoot,
} from './config.mjs';
import { assertFile, parsePrefixedJson } from './io.mjs';
import { buildSingleSummary, buildVariantSummary } from './summary.mjs';

async function bundleExampleBackends(bundleDir) {
  assertFile(backendBuildModulePath, 'volt-cli backend build output');
  assertFile(nodeBenchmarkScript, 'node benchmark source');
  mkdirSync(bundleDir, { recursive: true });

  const backendBuildModule = await import(pathToFileURL(backendBuildModulePath).href);
  const { buildBackendBundle } = backendBuildModule;
  if (typeof buildBackendBundle !== 'function') {
    throw new Error('[bench] buildBackendBundle export is unavailable.');
  }

  const bundleEnv = {};
  for (const project of Object.values(benchmarkProjects)) {
    await buildBackendBundle(
      project.projectDir,
      resolve(project.projectDir, project.backendEntry),
      project.bundlePath,
    );
    bundleEnv[project.envName] = project.bundlePath;
  }

  await buildBackendBundle(repoRoot, nodeBenchmarkScript, nodeBenchmarkBundlePath);

  return bundleEnv;
}

function runNodeBaseline(extraEnv = {}) {
  assertFile(nodeBenchmarkBundlePath, 'node benchmark bundle');
  const output = execFileSync('node', [nodeBenchmarkBundlePath], {
    cwd: repoRoot,
    encoding: 'utf8',
    env: { ...process.env, ...extraEnv },
    maxBuffer: 20 * 1024 * 1024,
  });
  return parsePrefixedJson(output, 'VOLT_NODE_BENCH_JSON:');
}

function runBoaBaseline(bundleEnv, extraEnv = {}) {
  const cargoArgs = [
    'test',
    '-p',
    'volt-runner',
    'headless_example_backends_emit_benchmark_summary',
  ];
  if (isReleaseMode) {
    cargoArgs.push('--release');
  }
  cargoArgs.push('--', '--ignored', '--nocapture');

  const output = execFileSync('cargo', cargoArgs, {
    cwd: repoRoot,
    encoding: 'utf8',
    env: { ...process.env, ...bundleEnv, ...extraEnv },
    maxBuffer: 20 * 1024 * 1024,
  });
  return parsePrefixedJson(output, 'VOLT_BENCH_JSON:');
}

function runBoaVariant(bundleEnv, engine, extraEnv = {}) {
  return runBoaBaseline(bundleEnv, {
    ...extraEnv,
    VOLT_BENCH_ENGINE: engine,
  });
}

function runBenchmarkSweep(bundleEnv) {
  return {
    generatedAt: new Date().toISOString(),
    platform: process.platform,
    mode: 'headless-backend-runtime',
    boaProfile: isReleaseMode ? 'release' : 'test',
    sweep: true,
    profiles: benchmarkSweepProfiles.map((profile) => {
      const overrideEnv = {
        VOLT_BENCH_PROFILE_JSON: JSON.stringify(profile.config),
      };
      const nodeSummary = runNodeBaseline(overrideEnv);
      const boaJsSummary = runBoaVariant(bundleEnv, 'js', overrideEnv);
      const boaNativeSummary = runBoaVariant(bundleEnv, 'native', overrideEnv);
      const directNativeSummary = runBoaVariant(bundleEnv, 'direct', overrideEnv);
      return {
        id: profile.id,
        config: profile.config,
        ...buildVariantSummary(nodeSummary, boaJsSummary, boaNativeSummary, directNativeSummary),
      };
    }),
  };
}

function runBenchmarkSummary(bundleEnv, isSweepMode) {
  if (isSweepMode) {
    return runBenchmarkSweep(bundleEnv);
  }

  return buildSingleSummary(
    isReleaseMode,
    runNodeBaseline(),
    runBoaVariant(bundleEnv, 'js'),
    runBoaVariant(bundleEnv, 'native'),
    runBoaVariant(bundleEnv, 'direct'),
  );
}

export { bundleExampleBackends, runBenchmarkSummary };
