#!/usr/bin/env bash
set -euo pipefail

run="${1:?Usage: download-binaries.sh CI_RUN_ID}"
temporary=$(mktemp -d "${TMPDIR:-/tmp}/get-next-version-binaries.XXXXXX")
trap 'rm -rf "$temporary"' EXIT
for platform in linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64 windows-arm64; do
  gh run download "$run" --name "binary-$platform" --dir "$temporary"
  suffix=''
  [[ "$platform" != windows-* ]] || suffix='.exe'
  # Artifact ZIPs do not preserve executable permissions.
  chmod +x "$temporary/get-next-version-$platform$suffix"
done
bash scripts/check-binaries.sh "$temporary"
mkdir -p bin
for platform in linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64 windows-arm64; do
  suffix=''
  [[ "$platform" != windows-* ]] || suffix='.exe'
  mv "$temporary/get-next-version-$platform$suffix" "bin/get-next-version-$platform$suffix"
  mv "$temporary/get-next-version-$platform$suffix.source" "bin/get-next-version-$platform$suffix.source"
done
bash scripts/check-binaries.sh
