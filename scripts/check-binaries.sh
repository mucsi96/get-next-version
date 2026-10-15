#!/usr/bin/env bash
set -euo pipefail

directory="${1:-bin}"

for platform in linux-x64 linux-arm64 macos-x64 macos-arm64 windows-x64 windows-arm64; do
  suffix=''
  [[ "$platform" != windows-* ]] || suffix='.exe'
  binary="$directory/get-next-version-$platform$suffix"
  if [[ ! -s "$binary" || ! -f "$binary.source" ]]; then
    printf 'Missing packaged binary or source manifest: %s\n' "$binary" >&2
    exit 1
  fi
  if ! diff -u "$binary.source" <(git hash-object Cargo.toml Cargo.lock rust-toolchain.toml src/main.rs); then
    printf 'Bundled binary is out of date: %s\n' "$binary" >&2
    exit 1
  fi
done
