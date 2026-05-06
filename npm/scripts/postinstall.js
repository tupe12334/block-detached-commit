#!/usr/bin/env node
'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');

const VERSION = require('../package.json').version;

const PLATFORM_MAP = {
  'linux-x64':   'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64':  'x86_64-apple-darwin',
  'darwin-arm64':'aarch64-apple-darwin',
  'win32-x64':   'x86_64-pc-windows-msvc',
};

const isWindows = process.platform === 'win32';
const platformKey = `${process.platform}-${process.arch}`;
const rustTarget = PLATFORM_MAP[platformKey];

if (!rustTarget) {
  console.warn(`block-detached-commit: unsupported platform ${platformKey} — binary not downloaded.`);
  console.warn('Install manually: cargo install block-detached-commit');
  process.exit(0);
}

const binaryName = isWindows ? 'block-detached-commit.exe' : 'block-detached-commit';
const assetName  = isWindows
  ? `block-detached-commit-${rustTarget}.exe`
  : `block-detached-commit-${rustTarget}`;

const destDir  = path.join(__dirname, '..', 'bin-native');
const destPath = path.join(destDir, binaryName);

const baseUrl = `https://github.com/tupe12334/block-detached-commit/releases/download/v${VERSION}`;

fs.mkdirSync(destDir, { recursive: true });

function download(url, dest, cb) {
  const file = fs.createWriteStream(dest);
  https.get(url, (res) => {
    if (res.statusCode === 301 || res.statusCode === 302) {
      file.close(() => download(res.headers.location, dest, cb));
      return;
    }
    if (res.statusCode !== 200) {
      file.close(() => {
        fs.unlink(dest, () => {});
        cb(new Error(`HTTP ${res.statusCode} for ${url}`));
      });
      return;
    }
    res.pipe(file);
    file.on('finish', () => file.close(cb));
    file.on('error', (err) => { fs.unlink(dest, () => {}); cb(err); });
  }).on('error', (err) => { fs.unlink(dest, () => {}); cb(err); });
}

function verifyChecksum(filePath, expectedHex, cb) {
  const crypto = require('crypto');
  const hash = crypto.createHash('sha256');
  const stream = fs.createReadStream(filePath);
  stream.on('data', (chunk) => hash.update(chunk));
  stream.on('end', () => {
    const actual = hash.digest('hex');
    cb(actual === expectedHex ? null : new Error(`checksum mismatch: expected ${expectedHex}, got ${actual}`));
  });
  stream.on('error', cb);
}

console.log(`block-detached-commit: downloading v${VERSION} for ${platformKey}...`);

download(`${baseUrl}/${assetName}`, destPath, (err) => {
  if (err) {
    console.warn(`block-detached-commit: download failed — ${err.message}`);
    console.warn('Install manually: cargo install block-detached-commit');
    process.exit(0);
  }

  // Verify SHA-256 checksum published alongside the release asset
  download(`${baseUrl}/${assetName}.sha256`, destPath + '.sha256', (err2) => {
    if (err2) {
      // Checksum file missing in dev/pre-release builds — skip verification
      if (!isWindows) fs.chmodSync(destPath, 0o755);
      console.log(`block-detached-commit: installed (checksum skipped — ${err2.message})`);
      return;
    }

    const expected = fs.readFileSync(destPath + '.sha256', 'utf8').trim().split(/\s+/)[0];
    fs.unlinkSync(destPath + '.sha256');

    verifyChecksum(destPath, expected, (err3) => {
      if (err3) {
        fs.unlinkSync(destPath);
        console.error(`block-detached-commit: ${err3.message}`);
        process.exit(1);
      }
      if (!isWindows) fs.chmodSync(destPath, 0o755);
      console.log(`block-detached-commit: installed at ${destPath}`);
    });
  });
});
