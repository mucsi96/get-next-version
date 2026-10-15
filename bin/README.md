# Bundled action binaries

These binaries are built from `src/main.rs` by the CI matrix and committed so
action consumers do not need Rust or a runtime download. Linux binaries use musl
and are statically linked. macOS and Windows binaries use their native targets.

Each `.source` file contains the Git blob hashes of `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, and `src/main.rs`, in that order. CI checks that all bundled
binaries correspond to the current source and runs the native one on each runner.

After a source change, push the branch, wait for CI to upload all six `binary-*`
artifacts, then run from the repository root:

```bash
bash scripts/download-binaries.sh <CI_RUN_ID>
git add bin
git commit -m "build: refresh bundled Rust binaries"
git push
```

The download helper stages all artifacts and checks their source manifests
before replacing existing bundled files; failed downloads leave `bin/` intact.

The initial CI run can fail its bundled-binary checks until the newly built
artifacts are committed; the build and upload steps run before those checks.
Require a green CI run before merging or publishing an action tag. Always publish
an action ref containing `action.yml`, `scripts/run.sh`, and all files in `bin/`.
