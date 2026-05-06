#!/usr/bin/env node
'use strict';

const { execFileSync, execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryName = process.platform === 'win32'
  ? 'block-detached-commit.exe'
  : 'block-detached-commit';

const localBinary = path.join(__dirname, '..', 'bin-native', binaryName);

let binary;

if (fs.existsSync(localBinary)) {
  binary = localBinary;
} else {
  // Fall back to binary already on PATH (e.g. installed via cargo or go)
  const finder = process.platform === 'win32' ? 'where' : 'which';
  try {
    execSync(`${finder} block-detached-commit`, { stdio: 'ignore' });
    binary = 'block-detached-commit';
  } catch {
    process.stderr.write(
      'block-detached-commit: binary not found.\n' +
      'Reinstall the package or run: cargo install block-detached-commit\n'
    );
    process.exit(2);
  }
}

try {
  execFileSync(binary, process.argv.slice(2), { stdio: 'inherit' });
} catch (e) {
  process.exit(typeof e.status === 'number' ? e.status : 1);
}
