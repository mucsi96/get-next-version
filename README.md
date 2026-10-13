# get-next-version

A GitHub Action that calculates the next version number based on git tags and source code changes.

The action is a composite action backed by bundled Rust binaries. Consumers do
not need a Rust toolchain, Node.js, or a binary download at runtime.

## How it works

1. Finds the previous tag reachable from HEAD matching `{prefix}-{N}`
2. If a tag exists, checks if source code has changed since that tag (using `git diff`)
3. If no changes detected → return the existing version with `hasNextVersion=false`
4. If changes detected → next version = highest existing version + 1
5. If no tags exist → version starts at 1

## Usage

```yaml
- uses: actions/checkout@v4

- name: Get next version
  id: get_next_version
  uses: mucsi96/get-next-version@main
  with:
    prefix: 'v'

- name: Create release
  if: steps.get_next_version.outputs.hasNextVersion == 'true'
  run: gh release create ${{ steps.get_next_version.outputs.tag }}
```

### With source directory and ignore paths

```yaml
- name: Get next version
  id: get_next_version
  uses: mucsi96/get-next-version@main
  with:
    prefix: 'app'
    src: 'src'
    ignore: 'src/generated docs'
```

The action fetches tags and full history automatically, including when checkout
is shallow. The checkout credentials must allow fetching from the repository.
Local repositories without remotes are also supported when their history is
complete. Git failures cause the action to fail instead of returning an incorrect
version.

`src` is a directory relative to the checkout. Ignore paths are Git pathspecs
relative to the checkout, separated by whitespace. Each is passed as its own
exclusion argument, so multiple ignored paths work together.

## Inputs

| Input    | Description                                             | Required | Default |
| -------- | ------------------------------------------------------- | -------- | ------- |
| `prefix` | Tag prefix (e.g. `v` for tags like `v-1`, `v-2`)       | Yes      |         |
| `src`    | Source directory to check for changes                   | No       | `.`     |
| `ignore` | Space-separated list of paths to ignore when detecting changes | No | `''`   |

## Outputs

| Output           | Description                                              |
| ---------------- | -------------------------------------------------------- |
| `version`        | The next version number when changed, or the current reachable release version when unchanged |
| `tag`            | The selected version tag including prefix, e.g. `v-2` |
| `hasNextVersion` | `true` if a new version should be released, else `false` |

`version` and `tag` are always populated. Use `hasNextVersion` to decide whether
to publish a new artifact, and use `version` to select the artifact for deployment
whether or not publication ran. Consumers do not need their own tag lookup.

| Situation | `version` | `tag` | `hasNextVersion` |
| --- | --- | --- | --- |
| No release tags yet | `1` | `server-1` | `true` |
| Server changed after `server-42` | `43` | `server-43` | `true` |
| Server unchanged since `server-42` | `42` | `server-42` | `false` |

For unchanged source, `version` uses the previous reachable component
tag, not a higher-numbered tag from an unrelated branch. New releases retain
the existing global highest-version-plus-one calculation.

### Conditional publication with deployment version selection

```yaml
- name: Get server version
  id: get_next_version
  uses: mucsi96/get-next-version@main
  with:
    prefix: 'server'
    src: 'server'

- name: Publish server image
  if: steps.get_next_version.outputs.hasNextVersion == 'true'
  uses: docker/build-push-action@v6
  with:
    context: server
    push: true
    tags: ghcr.io/${{ github.repository }}-server:${{ steps.get_next_version.outputs.version }}

- name: Deploy selected server version
  env:
    SERVER_IMAGE: ghcr.io/${{ github.repository }}-server:${{ steps.get_next_version.outputs.version }}
  run: bash scripts/deploy.sh
```

## Tag format

Tags follow the pattern `{prefix}-{N}` where `N` is a positive integer:

- `v-1`, `v-2`, `v-3`, ...
- `app-1`, `app-2`, `app-3`, ...

Only positive integer release tags are considered. Tags such as `v-0`, `v-01`,
and `v-2-beta` are ignored. Prefixes must form a valid Git tag name when followed
by `-1`.

## Supported runners

| Runner OS | Architectures | Binary target |
| --- | --- | --- |
| Linux | x64, ARM64 | Statically linked musl |
| macOS | x64, ARM64 | Native Darwin |
| Windows | x64, ARM64 | Native MSVC |

Git and Bash must be available (as they are on GitHub-hosted runners). The action
selects a binary from `bin/` using the runner OS and architecture and executes it
directly. Unsupported platforms fail with a clear error. All supported binaries
are included in the published action repository.

## Development and publishing

Install the toolchain pinned in `rust-toolchain.toml`, then run:

```bash
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
```

The implementation uses only the Rust standard library. Integration tests run the
binary against real Git repositories, including shallow checkouts, unrelated
release branches, component changes, ignored paths, and invalid inputs.

CI builds and tests all six native targets, uploads packaged binaries, validates
the committed binaries against their source manifests, and exercises the
composite action's output forwarding on every supported runner. To refresh the
bundled binaries after changing Rust source, follow [bin/README.md](bin/README.md).
Commit the refreshed artifacts and require green CI before merging or publishing
a release ref. Consumers then continue using the action with `uses:` and `with:`
as shown above, with no build step on their runner.

## License

[MIT](LICENSE)
