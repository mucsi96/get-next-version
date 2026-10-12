#!/usr/bin/env bash
set -euo pipefail

target="${1:?Usage: package.sh RUST_TARGET PLATFORM [EXE_SUFFIX]}"
platform="${2:?Usage: package.sh RUST_TARGET PLATFORM [EXE_SUFFIX]}"
suffix="${3:-}"

cargo build --release --locked --offline --target "$target"
mkdir -p bin
binary="bin/get-next-version-$platform$suffix"
cp "target/$target/release/get-next-version$suffix" "$binary"
chmod +x "$binary"
git hash-object Cargo.toml Cargo.lock rust-toolchain.toml src/main.rs > "$binary.source"
