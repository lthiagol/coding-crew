#!/usr/bin/env bash
# Tests for scripts/review-snapshot.sh.
#
# Builds a temporary git repository, snapshots a round's changes, changes the
# tree again, and proves the second snapshot's delta contains only the second
# round's hunks. No real repository is touched.
#
# Dependencies: bash, git, diff, coreutils.
set -uo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SNAPSHOT="$HERE/review-snapshot.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/review-snapshot-test.XXXXXX")
trap 'rm -rf "$TMP"' EXIT

PASS=0
FAIL=0

ok() { PASS=$((PASS + 1)); }
fail() {
  FAIL=$((FAIL + 1))
  printf 'FAIL: %s\n' "$*" >&2
}

check_line() {
  local file="$1" line="$2"
  if grep -Fqx -- "$line" "$file" 2>/dev/null; then
    ok
  else
    fail "$file missing line: $line"
  fi
}

check_no_line() {
  local file="$1" line="$2"
  if grep -Fqx -- "$line" "$file" 2>/dev/null; then
    fail "$file unexpectedly has line: $line"
  else
    ok
  fi
}

check_file() {
  local file="$1"
  if [ -f "$file" ]; then ok; else fail "missing file: $file"; fi
}

check_no_file() {
  local file="$1"
  if [ -e "$file" ]; then fail "unexpected file: $file"; else ok; fi
}

check_same() {
  local label="$1" before="$2" after="$3"
  if [ "$before" = "$after" ]; then
    ok
  else
    fail "$label changed"
  fi
}

check_contains() {
  local file="$1" pattern="$2"
  if grep -Fq -- "$pattern" "$file" 2>/dev/null; then
    ok
  else
    fail "$file missing text: $pattern"
  fi
}

check_absent() {
  local file="$1" pattern="$2"
  if grep -Fq -- "$pattern" "$file" 2>/dev/null; then
    fail "$file unexpectedly contains: $pattern"
  else
    ok
  fi
}

check_line_count() {
  local file="$1" expected="$2" actual
  actual=$(wc -l <"$file" | tr -d ' ')
  if [ "$actual" -eq "$expected" ]; then
    ok
  else
    fail "$file has $actual lines, expected $expected"
  fi
}

check_file_count() {
  local dir="$1" expected="$2" actual
  actual=$(find "$dir" -type f | wc -l | tr -d ' ')
  if [ "$actual" -eq "$expected" ]; then
    ok
  else
    fail "$dir holds $actual files, expected $expected"
  fi
}

REPO="$TMP/repo"
mkdir -p "$REPO"
cd "$REPO" || exit 1
git init -q
git config user.email test@example.com
git config user.name Test

printf 'alpha\n' >a.txt
printf 'bravo\n' >b.txt
printf 'charlie\n' >c.txt
printf 'delta\n' >d.txt
git add a.txt b.txt c.txt d.txt
git commit -qm initial

# Round 1: modify one file, add an untracked file, delete a tracked file.
printf 'alpha one\n' >a.txt
printf 'new file\n' >new.txt
rm c.txt

status_before=$(git status --porcelain=v1)
index_before=$(git diff --cached --name-only)
"$SNAPSHOT" "$TMP/snap1" >/dev/null
status_after=$(git status --porcelain=v1)
index_after=$(git diff --cached --name-only)

check_same "working tree status" "$status_before" "$status_after"
check_same "git index" "$index_before" "$index_after"

MANIFEST1="$TMP/snap1/manifest.txt"
check_file "$MANIFEST1"
check_line "$MANIFEST1" "changed a.txt"
check_line "$MANIFEST1" "added new.txt"
check_line "$MANIFEST1" "deleted c.txt"
check_no_line "$MANIFEST1" "changed b.txt"
check_line_count "$MANIFEST1" 3
check_file_count "$TMP/snap1/files" 2

check_file "$TMP/snap1/files/a.txt"
check_contains "$TMP/snap1/files/a.txt" "alpha one"
check_file "$TMP/snap1/files/new.txt"
check_no_file "$TMP/snap1/files/c.txt"
check_no_file "$TMP/snap1/files/b.txt"

# Round 2 fix: change a.txt again, touch a file that round 1 left clean, and
# delete another file round 1 did not touch.
printf 'alpha two\n' >a.txt
printf 'bravo two\n' >b.txt
rm d.txt
"$SNAPSHOT" "$TMP/snap2" >/dev/null

# A deletion made only in this round is recorded in the manifest and shows in
# the delta, even though the file appears in neither snapshot's file tree.
check_no_line "$MANIFEST1" "deleted d.txt"
check_line "$TMP/snap2/manifest.txt" "deleted d.txt"

# The first snapshot is a record and must not have moved with the tree.
check_line "$MANIFEST1" "changed a.txt"
check_contains "$TMP/snap1/files/a.txt" "alpha one"
check_absent "$TMP/snap1/files/a.txt" "alpha two"

"$SNAPSHOT" --diff "$TMP/snap1" "$TMP/snap2" >"$TMP/delta.diff"
DELTA="$TMP/delta.diff"

check_contains "$DELTA" "review-snapshot delta:"
check_contains "$DELTA" "a.txt"
check_contains "$DELTA" "-alpha one"
check_contains "$DELTA" "+alpha two"
check_contains "$DELTA" "b.txt"
check_contains "$DELTA" "+bravo two"
check_contains "$DELTA" "deleted d.txt"
check_absent "$DELTA" "files/new.txt"
check_absent "$DELTA" "files/c.txt"

# Paths with spaces survive the copy and the manifest.
mkdir -p "dir with space"
printf 'spaced\n' >"dir with space/f.txt"
"$SNAPSHOT" "$TMP/snap3" >/dev/null
check_line "$TMP/snap3/manifest.txt" "added dir with space/f.txt"
check_file "$TMP/snap3/files/dir with space/f.txt"

# Usage errors exit 2 and do not look like a successful snapshot.
"$SNAPSHOT" >/dev/null 2>&1
rc=$?
if [ "$rc" -eq 2 ]; then ok; else fail "missing snapshot directory should exit 2, got $rc"; fi

"$SNAPSHOT" --diff "$TMP/nope" "$TMP/snap2" >/dev/null 2>&1
rc=$?
if [ "$rc" -eq 2 ]; then ok; else fail "missing snapshot in --diff should exit 2, got $rc"; fi

printf 'review-snapshot: %d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
