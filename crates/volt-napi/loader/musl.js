/* eslint-disable */
// @ts-nocheck

const { readFileSync } = require('node:fs');
const { execSync } = require('node:child_process');

const isFileMusl = (filePath) => filePath.includes('libc.musl-') || filePath.includes('ld-musl-');

const isMuslFromFilesystem = () => {
  try {
    return readFileSync('/usr/bin/ldd', 'utf-8').includes('musl');
  } catch {
    return null;
  }
};

const isMuslFromReport = () => {
  let report = null;

  if (typeof process.report?.getReport === 'function') {
    process.report.excludeNetwork = true;
    report = process.report.getReport();
  }

  if (!report) {
    return null;
  }
  if (report.header && report.header.glibcVersionRuntime) {
    return false;
  }
  if (Array.isArray(report.sharedObjects) && report.sharedObjects.some(isFileMusl)) {
    return true;
  }

  return false;
};

const isMuslFromChildProcess = () => {
  try {
    return execSync('ldd --version', { encoding: 'utf8' }).includes('musl');
  } catch {
    return false;
  }
};

const isMusl = () => {
  if (process.platform !== 'linux') {
    return false;
  }

  return isMuslFromFilesystem() ?? isMuslFromReport() ?? isMuslFromChildProcess();
};

module.exports = {
  isMusl,
};
