#!/usr/bin/env bash
# Sync the third-party skills in this folder with their pinned upstream.
#
#   .agents/skills/sync.sh          copy the vendored skills, report adapted drift
#   .agents/skills/sync.sh --bump   move the global pin and its commit date,
#                                   then sync; the adapted skill keeps its own
#                                   source commit
#   .agents/skills/sync.sh --help   explain the two modes
#
# Vendored skills stay byte-identical to upstream: every file is copied from
# the pinned commit. code-review is adapted for coding-lanes and is never
# overwritten or auto-rebased: the script diffs it against its own recorded
# Source commit and reports that a bump needs a manual rebase. See UPSTREAM.md.

set -euo pipefail

# SYNC_REPO lets the tests drive the script against a local repository.
REPO="${SYNC_REPO:-https://github.com/mattpocock/skills}"
SKILLS_DIR="$(cd "$(dirname "$0")" && pwd)"
PIN_FILE="$SKILLS_DIR/UPSTREAM.md"
ADAPTED_PIN="$SKILLS_DIR/code-review/UPSTREAM.md"

# Vendored skills: <name>:<upstream path relative to the repo root>.
SKILLS=(
  "codebase-design:skills/engineering/codebase-design"
  "diagnosing-bugs:skills/engineering/diagnosing-bugs"
  "domain-modeling:skills/engineering/domain-modeling"
  "writing-for-agents:skills/productivity/writing-for-agents"
)

# Adapted skills: never overwritten, only diffed against their own source commit.
ADAPTED=(
  "code-review:skills/engineering/code-review"
)

usage() {
  cat <<'EOF'
usage: .agents/skills/sync.sh [--bump | --help]

  (no argument)  copy the vendored skills from the pinned commit; report what
                 changed, and report how the adapted code-review skill differs
                 from its pinned upstream
  --bump         move the vendored pin to the latest upstream main, refresh its
                 commit date, then sync; the adapted skill keeps its own
                 recorded source commit and is reported as needing a manual
                 rebase
  --help         show this text

Vendored skills are overwritten and stay byte-identical to upstream.
code-review is adapted for coding-lanes: it is never overwritten. Its own
recorded source commit is fetched and diffed against the local copy, and the
difference is printed as drift. --bump does not move that source commit.
EOF
}

pinned_commit() {
  grep -oE '`[0-9a-f]{40}`' "$PIN_FILE" | head -1 | tr -d '`'
}

adapted_source_commit() {
  [ -f "$ADAPTED_PIN" ] || return 0
  grep -E '^\| Source commit \|' "$ADAPTED_PIN" |
    grep -oE '`[0-9a-f]{40}`' | head -1 | tr -d '`'
}

sed_in_place() {
  local expr="$1" file="$2"
  if sed --version >/dev/null 2>&1; then
    sed -i -E "$expr" "$file"
  else
    sed -i '' -E "$expr" "$file"
  fi
}

fetch() {
  local commit="$1" tmp
  tmp="$(mktemp -d)"
  echo "fetching $REPO @ $commit" >&2
  git clone --quiet "$REPO" "$tmp/repo"
  git -C "$tmp/repo" checkout --quiet "$commit"
  echo "$tmp/repo"
}

copy_skill() {
  local src_root="$1" name="$2" up_path="$3"
  local src="$src_root/$up_path"
  local dst="$SKILLS_DIR/$name"

  mkdir -p "$dst"
  # Copy every file except the harness-specific agents/ directory.
  ( cd "$src" && find . -type f -not -path "./agents/*" -print0 ) |
    while IFS= read -r -d '' f; do
      mkdir -p "$dst/$(dirname "$f")"
      cp "$src/$f" "$dst/$f"
    done
}

report_diff() {
  local name="$1" up_path="$2" src_root="$3"
  local src="$src_root/$up_path" dst="$SKILLS_DIR/$name"
  if diff -rq "$src" "$dst" --exclude agents >/dev/null 2>&1; then
    echo "  $name: byte-identical to upstream"
  else
    echo "  $name differs from upstream:"
    diff -r "$src" "$dst" --exclude agents 2>/dev/null | grep "^[<>]" | head -20 || true
  fi
}

report_adapted() {
  local name="$1" up_path="$2" src_root="$3" source_commit="$4"
  local up_file="$src_root/$up_path/SKILL.md"
  local dst="$SKILLS_DIR/$name"
  local local_file="$dst/SKILL.md"
  local drift

  echo "  $name: adapted — local copy kept, not overwritten"
  echo "    source commit: $source_commit (its own record, not the global pin)"
  if [ ! -f "$local_file" ]; then
    echo "    local .agents/skills/$name/SKILL.md is missing; the adaptation is gone"
    return 0
  fi
  if [ ! -f "$up_file" ]; then
    echo "    upstream $up_path/SKILL.md is missing at this source commit"
    return 0
  fi
  if diff -q "$up_file" "$local_file" >/dev/null 2>&1; then
    echo "    matches upstream byte for byte; expected a coding-lanes adaptation"
    return 0
  fi

  drift="$(diff -u "$up_file" "$local_file" | grep -cE '^[+-][^+-]' || true)"
  echo "    upstream: $up_path/SKILL.md"
  echo "    local:    .agents/skills/$name/SKILL.md"
  echo "    drift:    $drift changed lines vs upstream"
  diff -u "$up_file" "$local_file" | sed -n '1,80p' || true
}

bump_pin() {
  local commit="$1" date="$2"
  if [ -z "$commit" ] || [ -z "$date" ]; then
    echo "sync.sh: could not read upstream main and its commit date" >&2
    exit 4
  fi
  if ! grep -qE '^\| Commit date \|' "$PIN_FILE"; then
    echo "sync.sh: no Commit date row in $PIN_FILE" >&2
    exit 4
  fi

  echo "bumping pin to $commit ($date)"
  sed_in_place "s/\`[0-9a-f]{40}\`/\`$commit\`/" "$PIN_FILE"
  sed_in_place "s/^\\| Commit date \\|.*\\|$/| Commit date | $date |/" "$PIN_FILE"

  if [ -f "$ADAPTED_PIN" ]; then
    # The adaptation stays based on the commit it records. Moving it is a
    # manual rebase, so --bump only reports the work and leaves the ledger.
    echo "adapted skill: code-review keeps its recorded Source commit"
    echo "  it needs a manual rebase onto $commit; see .agents/skills/code-review/UPSTREAM.md"
  fi
}

upstream_main_commit() {
  local ref
  ref="$(git ls-remote "$REPO" refs/heads/main 2>/dev/null)" || return 1
  printf '%s\n' "$ref" | cut -f1
}

upstream_commit_date() {
  local commit="$1" tmp date
  tmp="$(mktemp -d)"
  if ! git clone --quiet --no-checkout "$REPO" "$tmp/repo" 2>/dev/null; then
    rm -rf "$tmp"
    return 1
  fi
  if ! date="$(git -C "$tmp/repo" log -1 --format=%cs "$commit" 2>/dev/null)"; then
    date=""
  fi
  rm -rf "$tmp"
  [ -n "$date" ] || return 1
  printf '%s\n' "$date"
}

main() {
  local commit date
  case "${1:-}" in
    "")
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --bump)
      if ! commit="$(upstream_main_commit)"; then
        echo "sync.sh: could not read upstream main from $REPO" >&2
        exit 4
      fi
      if [ -z "$commit" ]; then
        echo "sync.sh: could not read upstream main from $REPO" >&2
        exit 4
      fi
      if ! date="$(upstream_commit_date "$commit")"; then
        echo "sync.sh: could not fetch upstream $commit to read its commit date" >&2
        exit 4
      fi
      bump_pin "$commit" "$date"
      ;;
    *)
      echo "sync.sh: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac

  commit="$(pinned_commit)"
  if [ -z "$commit" ]; then
    echo "sync.sh: no pinned commit in $PIN_FILE" >&2
    exit 4
  fi

  TMP_DIR="$(fetch "$commit")"
  ADAPTED_TMP=""
  trap 'rm -rf "${TMP_DIR:-}" "${ADAPTED_TMP:-}"' EXIT

  echo "vendored skills — copied and kept byte-identical:"
  for entry in "${SKILLS[@]}"; do
    local name="${entry%%:*}" up_path="${entry#*:}"
    copy_skill "$TMP_DIR" "$name" "$up_path"
    report_diff "$name" "$up_path" "$TMP_DIR"
  done

  local adapted_commit adapted_root
  adapted_commit="$(adapted_source_commit)"
  if [ -z "$adapted_commit" ]; then
    echo "sync.sh: no Source commit in $ADAPTED_PIN" >&2
    exit 4
  fi
  if [ "$adapted_commit" = "$commit" ]; then
    adapted_root="$TMP_DIR"
  else
    ADAPTED_TMP="$(fetch "$adapted_commit")"
    adapted_root="$ADAPTED_TMP"
  fi

  echo "adapted skills — local copy kept, upstream drift reported:"
  for entry in "${ADAPTED[@]}"; do
    local name="${entry%%:*}" up_path="${entry#*:}"
    report_adapted "$name" "$up_path" "$adapted_root" "$adapted_commit"
  done

  cp "$TMP_DIR/LICENSE" "$SKILLS_DIR/LICENSE"

  rm -rf "$TMP_DIR"
  [ -z "$ADAPTED_TMP" ] || rm -rf "$ADAPTED_TMP"
  trap - EXIT
  echo "done — provenance in .agents/skills/UPSTREAM.md"
}

main "$@"
