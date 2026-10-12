#!/usr/bin/env bash
set -euo pipefail

case "${ACTION_RUNNER_OS:?}/${ACTION_RUNNER_ARCH:?}" in
  Linux/X64) binary=get-next-version-linux-x64 ;;
  Linux/ARM64) binary=get-next-version-linux-arm64 ;;
  macOS/X64) binary=get-next-version-macos-x64 ;;
  macOS/ARM64) binary=get-next-version-macos-arm64 ;;
  Windows/X64) binary=get-next-version-windows-x64.exe ;;
  Windows/ARM64) binary=get-next-version-windows-arm64.exe ;;
  *)
    printf '::error::Unsupported runner: %s/%s\n' "$ACTION_RUNNER_OS" "$ACTION_RUNNER_ARCH" >&2
    exit 1
    ;;
esac

path="${ACTION_PATH:?}/bin/$binary"
if [[ ! -f "$path" ]]; then
  printf '::error::Bundled binary missing: %s\n' "$path" >&2
  exit 1
fi
exec "$path"
