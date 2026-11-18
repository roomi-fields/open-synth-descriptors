#!/usr/bin/env node
'use strict';

// Runs the binary that install.js downloaded, with the same arguments.

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const binary = path.join(__dirname, '..', 'vendor', process.platform === 'win32' ? 'osd.exe' : 'osd');
if (!fs.existsSync(binary)) {
  process.stderr.write('osd: the binary is missing; reinstall @roomi-fields/osd\n');
  process.exit(1);
}
const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  process.stderr.write(`osd: ${result.error.message}\n`);
  process.exit(1);
}
process.exit(result.status === null ? 1 : result.status);
