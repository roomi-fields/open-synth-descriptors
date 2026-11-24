'use strict';

// Downloads the binary of the running platform from the GitHub release of
// this package's own version, checks its SHA-256 against the release's
// SHA256SUMS file, and refuses a different checksum.

const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { currentAsset } = require('./platform.js');

const version = require('./package.json').version;
const base =
  process.env.OSD_RELEASE_BASE ||
  `https://github.com/roomi-fields/open-synth-descriptors/releases/download/v${version}`;

async function get(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url}: HTTP ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

async function main() {
  const asset = currentAsset();
  const sums = (await get(`${base}/SHA256SUMS`)).toString('utf8');
  const line = sums.split('\n').map((l) => l.trim().split(/\s+/)).find((p) => p[1] === asset);
  if (!line) {
    throw new Error(`SHA256SUMS has no line for ${asset}`);
  }
  const binary = await get(`${base}/${asset}`);
  const actual = crypto.createHash('sha256').update(binary).digest('hex');
  if (actual !== line[0].toLowerCase()) {
    throw new Error(`${asset}: checksum ${actual} differs from the release's ${line[0]}`);
  }
  const dir = path.join(__dirname, 'vendor');
  fs.mkdirSync(dir, { recursive: true });
  const target = path.join(dir, process.platform === 'win32' ? 'osd.exe' : 'osd');
  fs.writeFileSync(target, binary, { mode: 0o755 });
}

main().catch((error) => {
  process.stderr.write(`osd: install failed: ${error.message}\n`);
  process.exit(1);
});
