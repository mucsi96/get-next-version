#!/usr/bin/env bash
set -euo pipefail

run="${1:?Usage: download-binaries.sh CI_RUN_ID}"
mkdir -p bin
for platform in linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64 windows-arm64; do
  gh run download "$run" --name "binary-$platform" --dir bin
  suffix=''
  [[ "$platform" != windows-* ]] || suffix='.exe'
  # Artifact ZIPs do not preserve executable permissions.
  chmod +x "bin/get-next-version-$platform$suffix"
done
bash scripts/check-binaries.sh
