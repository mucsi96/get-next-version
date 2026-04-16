#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ORIG_DIR="$(pwd)"
PASS=0
FAIL=0

setup_repo() {
  local dir
  dir=$(mktemp -d)
  cd "$dir"
  git init -b main
  git config user.email "test@test.com"
  git config user.name "Test"
  git config commit.gpgsign false
  mkdir -p src
  echo "init" > src/main.txt
  git add .
  git commit --no-gpg-sign -m "initial commit"
}

cleanup_repo() {
  local d
  d=$(pwd)
  cd "$ORIG_DIR"
  rm -rf "$d"
}

run_action() {
  export INPUT_PREFIX="${1}"
  export INPUT_SRC="${2:-.}"
  export INPUT_IGNORE="${3:-}"
  local tmpout
  tmpout=$(mktemp)
  export GITHUB_OUTPUT="$tmpout"
  bash "$SCRIPT_DIR/get-next-version.sh" 2>&1
  cat "$tmpout"
  rm -f "$tmpout"
}

assert_output() {
  local label="$1" expected_version="$2" expected_has="$3" output="$4"
  local actual_version actual_has
  actual_version=$(echo "$output" | grep '^version=' | cut -d= -f2)
  actual_has=$(echo "$output" | grep '^hasNextVersion=' | cut -d= -f2)

  if [[ "$actual_version" == "$expected_version" && "$actual_has" == "$expected_has" ]]; then
    echo "  PASS: $label"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $label"
    echo "    expected version=$expected_version hasNextVersion=$expected_has"
    echo "    got      version=$actual_version hasNextVersion=$actual_has"
    FAIL=$((FAIL + 1))
  fi
}

commit() {
  echo "$RANDOM" >> src/main.txt
  git add .
  git commit --no-gpg-sign -m "$1"
}

# ─── Tests ────────────────────────────────────────────────────────────────────

echo "=== Test 1: No tags → version 1 ==="
setup_repo
OUT=$(run_action "app")
assert_output "no tags, first version" "1" "true" "$OUT"
cleanup_repo

echo "=== Test 2: Tag exists, no changes → no next version ==="
setup_repo
git tag "app-1"
OUT=$(run_action "app")
assert_output "tag exists, no changes" "" "false" "$OUT"
cleanup_repo

echo "=== Test 3: Tag exists, new commits → next version ==="
setup_repo
git tag "app-1"
commit "add feature"
OUT=$(run_action "app")
assert_output "tag 1, new commit" "2" "true" "$OUT"
cleanup_repo

echo "=== Test 4: Multiple tags, picks highest ==="
setup_repo
git tag "app-1"
commit "second"
git tag "app-2"
commit "third"
git tag "app-3"
commit "fourth"
OUT=$(run_action "app")
assert_output "highest tag is 3" "4" "true" "$OUT"
cleanup_repo

echo "=== Test 5: Different prefix ==="
setup_repo
git tag "v-1"
commit "add feature"
OUT=$(run_action "v")
assert_output "prefix v" "2" "true" "$OUT"
cleanup_repo

echo "=== Test 6: Prefix ignores non-matching tags ==="
setup_repo
git tag "other-10"
git tag "v-2"
commit "something"
OUT=$(run_action "v")
assert_output "prefix v ignores other-10" "3" "true" "$OUT"
cleanup_repo

echo "=== Test 7: src directory scoping ==="
setup_repo
git tag "app-1"
mkdir -p docs
echo "readme" > docs/readme.txt
git add .
git commit --no-gpg-sign -m "update docs"
OUT=$(run_action "app" "src")
assert_output "only docs changed, src unchanged" "" "false" "$OUT"
cleanup_repo

echo "=== Test 8: src directory scoping with actual changes ==="
setup_repo
git tag "app-1"
commit "change source"
OUT=$(run_action "app" "src")
assert_output "src changed" "2" "true" "$OUT"
cleanup_repo

echo "=== Test 9: ignore paths ==="
setup_repo
git tag "app-1"
echo "generated" > src/generated.txt
git add .
git commit --no-gpg-sign -m "update generated file"
OUT=$(run_action "app" "." "src/generated.txt")
assert_output "ignored file changed" "" "false" "$OUT"
cleanup_repo

# ─── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════"
echo "  Results: $PASS passed, $FAIL failed"
echo "═══════════════════════════════════"

[[ "$FAIL" -eq 0 ]] && exit 0 || exit 1
