#!/usr/bin/env bash
set -euo pipefail

PREFIX="${INPUT_PREFIX}"
SRC="${INPUT_SRC:-.}"
IGNORE="${INPUT_IGNORE:-}"

# Ensure we have the full history and all tags
git fetch --tags --unshallow 2>/dev/null || git fetch --tags 2>/dev/null || true

# Find previous tag reachable from HEAD
PREV_TAG=$(git describe --tags --match="${PREFIX}-[1-9]*" --abbrev=0 2>/dev/null || true)

if [[ -n "$PREV_TAG" ]]; then
  echo "Previous tag: $PREV_TAG"

  # Check if source code has changed since previous tag
  IGNORE_ARGS=()
  for path in $IGNORE; do
    IGNORE_ARGS+=(":!${path}")
  done

  echo "Detecting changes in ${SRC} since ${PREV_TAG}"
  DIFF_OUTPUT=$(git diff --name-only HEAD "$PREV_TAG" -- "$SRC" "${IGNORE_ARGS[@]+"${IGNORE_ARGS[@]}"}" 2>/dev/null || true)

  if [[ -n "$DIFF_OUTPUT" ]]; then
    echo "$DIFF_OUTPUT"
  else
    VERSION="${PREV_TAG#"${PREFIX}-"}"
    echo "No changes detected since ${PREFIX}:${VERSION} in ${SRC}."
    {
      echo "version="
      echo "hasNextVersion=false"
    } >> "${GITHUB_OUTPUT:-/dev/null}"
    exit 0
  fi
fi

# Get latest version from all tags matching the prefix
LATEST_TAG=$(git tag --list --sort=-v:refname "${PREFIX}-[1-9]*" | head -n1 || true)

if [[ -n "$LATEST_TAG" ]]; then
  LATEST_VERSION="${LATEST_TAG#"${PREFIX}-"}"
  NEW_VERSION=$((LATEST_VERSION + 1))
else
  NEW_VERSION=1
fi

echo "Changes detected for ${PREFIX}. New version: ${NEW_VERSION}"

{
  echo "version=${NEW_VERSION}"
  echo "hasNextVersion=true"
} >> "${GITHUB_OUTPUT:-/dev/null}"
