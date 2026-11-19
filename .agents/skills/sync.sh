#!/usr/bin/env bash
# Sync the third-party skills in this folder with their pinned upstream.
#
#   .agents/skills/sync.sh          copy the vendored skills, report adapted drift
#   .agents/skills/sync.sh --bump   move the global pin and its commit date,
#                                   then sync; each adapted skill keeps its own
#                                   source commit
#   .agents/skills/sync.sh --help   explain the two modes
#
# Vendored skills stay byte-identical to upstream: every file is copied from
# the pinned commit. Adapted skills are never overwritten or auto-rebased: each
# entry names its own local directory and reads the pin from that directory's
# UPSTREAM.md, so an adapted skill may live outside this repository. The script
# diffs it against its recorded Source commit and reports that a bump needs a
# manual rebase. See UPSTREAM.md.
#
# code-review left the adapted list: it now lives globally at
# ~/.config/opencode/skills/code-review and is maintained by hand. The
# mechanism stays for a future adapted skill.

set -euo pipefail

# SYNC_REPO lets the tests drive the script against a local repository.
REPO="${SYNC_REPO:-https://github.com/mattpocock/skills}"
SKILLS_DIR="$(cd "$(dirname "$0")" && pwd)"
PIN_FILE="$SKILLS_DIR/UPSTREAM.md"

# Vendored skills: <name>:<upstream path relative to the repo root>.
SKILLS=(
  "codebase-design:skills/engineering/codebase-design"
  "diagnosing-bugs:skills/engineering/diagnosing-bugs"
  "domain-modeling:skills/engineering/domain-modeling"
  "writing-for-agents:skills/productivity/writing-for-agents"
)

# Adapted skills: never overwritten, only diffed against their own source
# commit. Each entry is <local directory>:<upstream path relative to the repo
# root>. The local directory is absolute or relative to this folder; it holds
# SKILL.md and UPSTREAM.md, which records that entry's own Source commit, and it
# may live outside this repository. The list is empty today: code-review left it
# when it moved to ~/.config/opencode/skills/code-review and became maintained
# by hand.
ADAPTED=(
)

# SYNC_ADAPTED lets the tests declare adapted entries without editing the
# script. One entry per line, same form as above.
if [ -n "${SYNC_ADAPTED:-}" ]; then
  mapfile -t ADAPTED <<<"$SYNC_ADAPTED"
fi

usage() {
  cat <<'EOF'
usage: .agents/skills/sync.sh [--bump | --help]

  (no argument)  copy the vendored skills from the pinned commit; report what
                 changed, and report how any adapted skill differs from its
                 pinned upstream
  --bump         move the vendored pin to the latest upstream main, refresh its
                 commit date, then sync; an adapted skill keeps its own
                 recorded source commit and is reported as needing a manual
                 rebase
  --help         show this text

Vendored skills are overwritten and stay byte-identical to upstream.
Adapted skills are never overwritten: each entry names its own local directory
and reads the pin from its UPSTREAM.md. The script fetches that source commit,
diffs the local copy against it, and prints the difference as drift. --bump
does not move those source commits. The adapted list is empty today:
code-review moved to ~/.config/opencode/skills/code-review and is maintained
by hand.
EOF
}

pinned_commit() {
  grep -oE '`[0-9a-f]{40}`' "$PIN_FILE" | head -1 | tr -d '`'
}

adapted_source_commit() {
  local pin_file="$1"
  [ -f "$pin_file" ] || return 0
  grep -E '^\| Source commit \|' "$pin_file" |
    grep -oE '`[0-9a-f]{40}`' | head -1 | tr -d '`'
}

adapted_dir() {
  case "$1" in
    /*) printf '%s\n' "$1" ;;
    *) printf '%s\n' "$SKILLS_DIR/$1" ;;
  esac
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
  if ! git clone --quiet "$REPO" "$tmp/repo" 2>/dev/null; then
    rm -rf "$tmp"
    echo "sync.sh: could not read upstream from $REPO" >&2
    exit 4
  fi
  if ! git -C "$tmp/repo" checkout --quiet "$commit" 2>/dev/null; then
    rm -rf "$tmp"
    echo "sync.sh: could not fetch upstream $commit from $REPO" >&2
    exit 4
  fi
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
  local local_dir="$1" up_path="$2" src_root="$3" source_commit="$4"
  local name
  name="$(basename "$local_dir")"
  local up_file="$src_root/$up_path/SKILL.md"
  local local_file="$local_dir/SKILL.md"
  local drift

  echo "  $name: adapted — local copy kept, not overwritten"
  echo "    local dir: $local_dir"
  echo "    source commit: $source_commit (its own record, not the global pin)"
  if [ ! -f "$local_file" ]; then
    echo "    $local_file is missing; the adaptation is gone"
    return 0
  fi
  if [ ! -f "$up_file" ]; then
    echo "    upstream $up_path/SKILL.md is missing at this source commit"
    return 0
  fi
  if diff -q "$up_file" "$local_file" >/dev/null 2>&1; then
    echo "    matches upstream byte for byte; expected an adaptation"
    return 0
  fi

  drift="$(diff -u "$up_file" "$local_file" | grep -cE '^[+-][^+-]' || true)"
  echo "    upstream: $up_path/SKILL.md"
  echo "    local:    $local_file"
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

  if [ "${#ADAPTED[@]}" -gt 0 ]; then
    # Each adaptation stays based on the commit its own ledger records. Moving
    # it is a manual rebase, so --bump only reports the work and leaves it.
    local entry local_dir
    echo "adapted skills keep their recorded Source commits:"
    for entry in "${ADAPTED[@]}"; do
      local_dir="$(adapted_dir "${entry%%:*}")"
      echo "  $(basename "$local_dir") needs a manual rebase onto $commit; see $local_dir/UPSTREAM.md"
    done
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
  ADAPTED_TMP_DIRS=()
  cleanup() {
    local d
    if [ -n "${TMP_DIR:-}" ]; then rm -rf "$TMP_DIR"; fi
    for d in ${ADAPTED_TMP_DIRS[@]+"${ADAPTED_TMP_DIRS[@]}"}; do
      if [ -n "$d" ]; then rm -rf "$d"; fi
    done
  }
  trap cleanup EXIT

  echo "vendored skills — copied and kept byte-identical:"
  for entry in "${SKILLS[@]}"; do
    local name="${entry%%:*}" up_path="${entry#*:}"
    copy_skill "$TMP_DIR" "$name" "$up_path"
    report_diff "$name" "$up_path" "$TMP_DIR"
  done

  echo "adapted skills — local copy kept, upstream drift reported:"
  if [ "${#ADAPTED[@]}" -eq 0 ]; then
    echo "  (none)"
  fi
  local local_dir up_path adapted_commit adapted_root
  for entry in "${ADAPTED[@]}"; do
    case "$entry" in
      *:*) ;;
      *)
        echo "sync.sh: malformed adapted entry: $entry" >&2
        exit 4
        ;;
    esac
    up_path="${entry#*:}"
    if [ -z "${entry%%:*}" ] || [ -z "$up_path" ]; then
      echo "sync.sh: malformed adapted entry: $entry" >&2
      exit 4
    fi
    local_dir="$(adapted_dir "${entry%%:*}")"
    adapted_commit="$(adapted_source_commit "$local_dir/UPSTREAM.md")"
    if [ -z "$adapted_commit" ]; then
      echo "sync.sh: no Source commit in $local_dir/UPSTREAM.md" >&2
      exit 4
    fi
    if [ "$adapted_commit" = "$commit" ]; then
      adapted_root="$TMP_DIR"
    else
      adapted_root="$(fetch "$adapted_commit")"
      ADAPTED_TMP_DIRS+=("$adapted_root")
    fi
    report_adapted "$local_dir" "$up_path" "$adapted_root" "$adapted_commit"
  done

  cp "$TMP_DIR/LICENSE" "$SKILLS_DIR/LICENSE"

  echo "done — provenance in .agents/skills/UPSTREAM.md"
}

main "$@"
