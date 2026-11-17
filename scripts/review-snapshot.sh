#!/usr/bin/env bash
# Snapshot the uncommitted change for a review round.
#
#   scripts/review-snapshot.sh <snapshot-dir>
#   scripts/review-snapshot.sh --diff <previous-dir> <current-dir>
#
# A snapshot copies every changed tracked file and every untracked file into
# <snapshot-dir>/files/, and writes <snapshot-dir>/manifest.txt with one
# `changed`, `added`, or `deleted` line per path. It reads git and copies files
# only: the real index and working tree are never modified.
#
# Run from the repository root. The orchestrator takes a snapshot after every
# builder round; a fix reviewer reads the --diff of the previous and current
# snapshots, because `git diff` still shows the first round's change.

set -euo pipefail

usage() {
  cat <<'EOF'
usage: scripts/review-snapshot.sh <snapshot-dir>
       scripts/review-snapshot.sh --diff <previous-dir> <current-dir>
       scripts/review-snapshot.sh --help

  <snapshot-dir>   copy the changed and untracked files there and write
                   manifest.txt: changed, added and deleted paths
  --diff           print the unified diff of two snapshots' file trees and
                   manifests; only what differs between the two rounds appears
  --help           show this text

Run from the repository root. The snapshot never stages, unstages, checks out
or edits anything: the real index and working tree stay as they were.
EOF
}

die_usage() {
  echo "review-snapshot: $1" >&2
  usage >&2
  exit 2
}

copy_file() {
  local path="$1"
  [ -e "$path" ] || [ -L "$path" ] || return 0
  mkdir -p "$OUT/files/$(dirname -- "$path")"
  cp -P -- "$path" "$OUT/files/$path"
}

snapshot() {
  local out="$1"
  local root out_abs out_rel
  root="$(git rev-parse --show-toplevel 2>/dev/null)" ||
    die_usage "not inside a git repository"
  cd "$root"

  mkdir -p "$out/files"
  OUT="$(cd "$out" && pwd -P)"
  out_abs="$OUT"
  out_rel=""
  case "$out_abs" in
    "$root"/*) out_rel="${out_abs#"$root"/}" ;;
  esac

  local manifest_tmp="$OUT/.manifest.$$"
  : >"$manifest_tmp"
  trap 'rm -f "$manifest_tmp"' RETURN

  local -a fields=()
  local field st path old new p
  while IFS= read -r -d '' field; do
    fields+=("$field")
  done < <(git diff --name-status -z HEAD 2>/dev/null || true)

  local i=0
  while ((i < ${#fields[@]})); do
    st="${fields[i]}"
    i=$((i + 1))
    case "$st" in
      R* | C*)
        old="${fields[i]}"
        new="${fields[i + 1]}"
        i=$((i + 2))
        printf 'deleted %s\n' "$old" >>"$manifest_tmp"
        printf 'added %s\n' "$new" >>"$manifest_tmp"
        copy_file "$new"
        ;;
      A*)
        p="${fields[i]}"
        i=$((i + 1))
        printf 'added %s\n' "$p" >>"$manifest_tmp"
        copy_file "$p"
        ;;
      D*)
        p="${fields[i]}"
        i=$((i + 1))
        printf 'deleted %s\n' "$p" >>"$manifest_tmp"
        ;;
      *)
        p="${fields[i]}"
        i=$((i + 1))
        if [ -e "$p" ] || [ -L "$p" ]; then
          printf 'changed %s\n' "$p" >>"$manifest_tmp"
          copy_file "$p"
        else
          printf 'deleted %s\n' "$p" >>"$manifest_tmp"
        fi
        ;;
    esac
  done

  while IFS= read -r -d '' p; do
    if [ -n "$out_rel" ]; then
      case "$p" in
        "$out_rel" | "$out_rel"/*) continue ;;
      esac
    fi
    printf 'added %s\n' "$p" >>"$manifest_tmp"
    copy_file "$p"
  done < <(git ls-files --others --exclude-standard -z)

  LC_ALL=C sort "$manifest_tmp" >"$OUT/manifest.txt"
  rm -f "$manifest_tmp"
  trap - RETURN
}

diff_snapshots() {
  local previous="$1" current="$2" rc
  [ -d "$previous/files" ] || die_usage "not a snapshot: $previous"
  [ -d "$current/files" ] || die_usage "not a snapshot: $current"

  echo "review-snapshot delta: $previous -> $current"
  if [ -f "$previous/manifest.txt" ] && [ -f "$current/manifest.txt" ] &&
    ! diff -q "$previous/manifest.txt" "$current/manifest.txt" >/dev/null 2>&1; then
    echo "manifest changes:"
    diff -u \
      --label "previous/manifest.txt" \
      --label "current/manifest.txt" \
      "$previous/manifest.txt" "$current/manifest.txt" || true
  fi
  set +e
  diff -ruN -- "$previous/files" "$current/files"
  rc=$?
  set -e
  # 0 is equal, 1 is a difference; anything else is a diff failure.
  [ "$rc" -le 1 ] || exit "$rc"
}

case "${1:-}" in
  --help | -h)
    usage
    ;;
  --diff)
    [ "$#" -eq 3 ] || die_usage "--diff needs two snapshot directories"
    diff_snapshots "$2" "$3"
    ;;
  "")
    die_usage "missing snapshot directory"
    ;;
  -*)
    die_usage "unknown option: $1"
    ;;
  *)
    [ "$#" -eq 1 ] || die_usage "takes one snapshot directory"
    snapshot "$1"
    ;;
esac
