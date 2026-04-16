# get-next-version

A GitHub Action that calculates the next version number based on git tags and source code changes.

## How it works

1. Finds the previous tag reachable from HEAD matching `{prefix}-{N}`
2. If a tag exists, checks if source code has changed since that tag (using `git diff`)
3. If no changes detected → no next version
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
  run: echo "Releasing version ${{ steps.get_next_version.outputs.version }}"
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
| `version`        | The next version number (empty if no changes detected)   |
| `hasNextVersion` | `true` if a new version should be released, else `false` |

## Tag format

Tags follow the pattern `{prefix}-{N}` where `N` is a positive integer:

- `v-1`, `v-2`, `v-3`, ...
- `app-1`, `app-2`, `app-3`, ...

## License

[MIT](LICENSE)
