'use strict';

// The four platforms the release builds, each with the name of its asset.
const ASSETS = {
  'linux-x64': 'osd-x86_64-unknown-linux-gnu',
  'darwin-arm64': 'osd-aarch64-apple-darwin',
  'darwin-x64': 'osd-x86_64-apple-darwin',
  'win32-x64': 'osd-x86_64-pc-windows-msvc.exe',
};

// The asset of a platform ("<process.platform>-<process.arch>"), or null.
function assetFor(platform) {
  return Object.prototype.hasOwnProperty.call(ASSETS, platform) ? ASSETS[platform] : null;
}

// The asset of the running platform; any other is refused with exit code 2.
function currentAsset() {
  const platform = `${process.platform}-${process.arch}`;
  const asset = assetFor(platform);
  if (asset === null) {
    process.stderr.write(`osd: no binary for ${platform}; supported: ${Object.keys(ASSETS).join(', ')}\n`);
    process.exit(2);
  }
  return asset;
}

module.exports = { ASSETS, assetFor, currentAsset };
