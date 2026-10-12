# get-next-version

A GitHub Action that calculates the next version number based on git tags and source code changes.

## How it works

1. Finds the previous tag reachable from HEAD matching `{prefix}-{N}`
2. If a tag exists, checks if source code has changed since that tag (using `git diff`)
3. If no changes detected → return the existing version with `hasNextVersion=false`
4. If changes detected → next version = highest existing version + 1
5. If no tags exist → version starts at 1

## Usage

```yaml
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

## License

[MIT](LICENSE)
