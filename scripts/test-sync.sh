#!/usr/bin/env bash
# Tests for .agents/skills/sync.sh.
#
# Builds a temporary upstream repository and a temporary skills folder, then
# drives sync.sh against them through SYNC_REPO. The tests cover the bump's
# commit-date refresh, the adapted ledger staying put, and the script's own
# exit 4 when upstream cannot be read. No network and no real tree is touched.
#
# Dependencies: bash, git, diff, coreutils.
set -uo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SYNC_SRC="$HERE/../.agents/skills/sync.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/sync-test.XXXXXX")
trap 'rm -rf "$TMP"' EXIT

PASS=0
FAIL=0

ok() { PASS=$((PASS + 1)); }
fail() {
  FAIL=$((FAIL + 1))
  printf 'FAIL: %s\n' "$*" >&2
}

check_eq() {
  local label="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    ok
  else
    fail "$label: expected '$expected', got '$actual'"
  fi
}

check_line() {
  local file="$1" line="$2"
  if grep -Fqx -- "$line" "$file" 2>/dev/null; then
    ok
  else
    fail "$file missing line: $line"
  fi
}

check_contains() {
  local file="$1" text="$2"
  if grep -Fq -- "$text" "$file" 2>/dev/null; then
    ok
  else
    fail "$file missing text: $text"
  fi
}

check_absent() {
  local file="$1" text="$2"
  if grep -Fq -- "$text" "$file" 2>/dev/null; then
    fail "$file unexpectedly contains: $text"
  else
    ok
  fi
}

check_same_file() {
  local label="$1" before="$2" after="$3"
  if cmp -s "$before" "$after"; then
    ok
  else
    fail "$label changed"
  fi
}

write_skills() {
  local dest="$1" pin="$2" source="$3"
  mkdir -p "$dest/code-review"
  cp "$SYNC_SRC" "$dest/sync.sh"
  cat >"$dest/UPSTREAM.md" <<EOF
# Upstream provenance

| | |
|---|---|
| Repository | \`github.com/mattpocock/skills\` |
| Pinned commit | \`$pin\` |
| Commit date | 2026-01-01 |
| Vendored on | 2026-01-02 |
EOF
  cat >"$dest/code-review/UPSTREAM.md" <<EOF
# Upstream provenance: code-review

| | |
|---|---|
| Source commit | \`$source\` |
| Adapted on | 2026-01-03 |
EOF
  printf 'local adapted copy\n' >"$dest/code-review/SKILL.md"
}

UPSTREAM="$TMP/upstream"
mkdir -p "$UPSTREAM"
git init -q "$UPSTREAM"
git -C "$UPSTREAM" symbolic-ref HEAD refs/heads/main
git -C "$UPSTREAM" config user.email test@example.com
git -C "$UPSTREAM" config user.name Test

mkdir -p "$UPSTREAM/skills/engineering/codebase-design"
mkdir -p "$UPSTREAM/skills/engineering/diagnosing-bugs"
mkdir -p "$UPSTREAM/skills/engineering/domain-modeling"
mkdir -p "$UPSTREAM/skills/productivity/writing-for-agents"
mkdir -p "$UPSTREAM/skills/engineering/code-review"
printf 'codebase-design upstream\n' >"$UPSTREAM/skills/engineering/codebase-design/SKILL.md"
printf 'diagnosing-bugs upstream\n' >"$UPSTREAM/skills/engineering/diagnosing-bugs/SKILL.md"
printf 'domain-modeling upstream\n' >"$UPSTREAM/skills/engineering/domain-modeling/SKILL.md"
printf 'writing-for-agents upstream\n' >"$UPSTREAM/skills/productivity/writing-for-agents/SKILL.md"
printf 'code-review upstream\n' >"$UPSTREAM/skills/engineering/code-review/SKILL.md"
printf 'MIT\n' >"$UPSTREAM/LICENSE"

git -C "$UPSTREAM" add -A
GIT_AUTHOR_DATE="2026-01-01T12:00:00 +0000" \
  GIT_COMMITTER_DATE="2026-01-01T12:00:00 +0000" \
  git -C "$UPSTREAM" commit -qm "upstream source"
SOURCE_COMMIT=$(git -C "$UPSTREAM" rev-parse HEAD)

printf 'main change\n' >"$UPSTREAM/CHANGELOG"
git -C "$UPSTREAM" add -A
GIT_AUTHOR_DATE="2026-05-02T12:00:00 +0000" \
  GIT_COMMITTER_DATE="2026-05-02T12:00:00 +0000" \
  git -C "$UPSTREAM" commit -qm "upstream main"
MAIN_COMMIT=$(git -C "$UPSTREAM" rev-parse HEAD)
MAIN_DATE=$(git -C "$UPSTREAM" log -1 --format=%cs "$MAIN_COMMIT")

SKILLS="$TMP/skills"
write_skills "$SKILLS" "$SOURCE_COMMIT" "$SOURCE_COMMIT"
cp "$SKILLS/code-review/SKILL.md" "$TMP/adapted-before"
cp "$SKILLS/code-review/UPSTREAM.md" "$TMP/adapted-ledger-before"

if SYNC_REPO="$UPSTREAM" bash "$SKILLS/sync.sh" --bump >"$TMP/bump.out" 2>"$TMP/bump.err"; then
  ok
else
  fail "bump failed: $(cat "$TMP/bump.err")"
fi

check_line "$SKILLS/UPSTREAM.md" "| Pinned commit | \`$MAIN_COMMIT\` |"
check_line "$SKILLS/UPSTREAM.md" "| Commit date | $MAIN_DATE |"
check_contains "$TMP/bump.out" "bumping pin to $MAIN_COMMIT ($MAIN_DATE)"
check_absent "$SKILLS/UPSTREAM.md" "2026-01-01"

check_contains "$SKILLS/code-review/UPSTREAM.md" "$SOURCE_COMMIT"
check_absent "$SKILLS/code-review/UPSTREAM.md" "$MAIN_COMMIT"
check_same_file "adapted ledger" "$TMP/adapted-ledger-before" "$SKILLS/code-review/UPSTREAM.md"
check_same_file "adapted SKILL.md" "$TMP/adapted-before" "$SKILLS/code-review/SKILL.md"

for skill in codebase-design diagnosing-bugs domain-modeling; do
  check_same_file "$skill" \
    "$UPSTREAM/skills/engineering/$skill/SKILL.md" \
    "$SKILLS/$skill/SKILL.md"
done
check_same_file "writing-for-agents" \
  "$UPSTREAM/skills/productivity/writing-for-agents/SKILL.md" \
  "$SKILLS/writing-for-agents/SKILL.md"
check_same_file "LICENSE" "$UPSTREAM/LICENSE" "$SKILLS/LICENSE"

# A repository that cannot be read at all: the script's own message and exit 4,
# not git's raw 128, and the pin stays untouched.
BAD_SKILLS="$TMP/bad-skills"
write_skills "$BAD_SKILLS" "$SOURCE_COMMIT" "$SOURCE_COMMIT"
cp "$BAD_SKILLS/UPSTREAM.md" "$TMP/bad-pin-before"
if SYNC_REPO="$TMP/not-a-repository" bash "$BAD_SKILLS/sync.sh" --bump \
  >"$TMP/bad.out" 2>"$TMP/bad.err"; then
  fail "unreadable repository should fail"
else
  rc=$?
  check_eq "unreadable repository exit" "4" "$rc"
fi
check_contains "$TMP/bad.err" "sync.sh: could not read upstream main"
check_absent "$TMP/bad.err" "fatal:"
check_same_file "pin after failed bump" "$TMP/bad-pin-before" "$BAD_SKILLS/UPSTREAM.md"

# A repository with no main branch: ls-remote succeeds but reads nothing, so
# the same error path fires instead of a hash-less bump.
EMPTY="$TMP/empty-upstream"
mkdir -p "$EMPTY"
git init -q "$EMPTY"
git -C "$EMPTY" symbolic-ref HEAD refs/heads/main
EMPTY_SKILLS="$TMP/empty-skills"
write_skills "$EMPTY_SKILLS" "$SOURCE_COMMIT" "$SOURCE_COMMIT"
cp "$EMPTY_SKILLS/UPSTREAM.md" "$TMP/empty-pin-before"
if SYNC_REPO="$EMPTY" bash "$EMPTY_SKILLS/sync.sh" --bump \
  >"$TMP/empty.out" 2>"$TMP/empty.err"; then
  fail "empty upstream should fail"
else
  rc=$?
  check_eq "empty upstream exit" "4" "$rc"
fi
check_contains "$TMP/empty.err" "sync.sh: could not read upstream main"
check_same_file "pin after empty upstream" "$TMP/empty-pin-before" "$EMPTY_SKILLS/UPSTREAM.md"

printf 'sync: %d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
