#!/usr/bin/env bash
# Close the Herdr tab a pane lives in, and prove the close.
#
# Usage:
#   scripts/herdr-close-pane.sh <pane-id>
#   scripts/herdr-close-pane.sh --audit <workspace-id>
#
# `herdr tab close` takes a TAB id (wX:t7); the orchestrator holds a PANE id
# (wX:p2). A close issued with the pane id answers tab_not_found, which reads
# like "already closed", and the tab leaks - and a leaked tab keeps the
# workspace alive and holds an opencode client, its own --standalone server and
# a pty daemon.
#
# This command takes the pane id, resolves the tab id with `herdr pane get`,
# closes the tab, and proves the close: the tab no longer appears in
# `herdr tab list --workspace <ID>`. It prints the workspace's remaining tabs
# with their agent state, so a leak surfaces. --audit prints that listing and
# closes nothing.
#
# Exit codes:
#   0  the close is proved: the tab is gone from the workspace's listing
#   2  usage error
#   3  the pane id resolves to no tab
#   4  the close was refused, the tab is still listed, or a Herdr call failed
#
# A refused close is exit 4 even when the listing looks clean: the refusal says
# the close did not happen. The one listing that proves a close without tabs is
# a workspace herdr reports gone - the resolved pane named it a moment ago, so
# its last tab went with it.
#
# Every Herdr call runs under a hard bound (HERDR_CALL_TIMEOUT, default 10s):
# SIGKILL keeps a hung client from hanging the closer. stdout carries the JSON
# response and is the only stream parsed. stderr is kept apart, so a warning
# line cannot break the parse; herdr's error envelope is read from it only when
# the call itself fails.
#
# Dependencies: bash, python3, herdr, timeout and mktemp (coreutils).
set -uo pipefail

CALL_TIMEOUT=${HERDR_CALL_TIMEOUT:-10}
[[ "$CALL_TIMEOUT" =~ ^[1-9][0-9]*$ ]] || CALL_TIMEOUT=10
ERR_FILE=$(mktemp "${TMPDIR:-/tmp}/herdr-close-pane.XXXXXX")
trap 'rm -f "$ERR_FILE"' EXIT

AUDIT=0
POSITIONAL=()

usage() {
  cat <<'EOF'
Usage: scripts/herdr-close-pane.sh <pane-id>
       scripts/herdr-close-pane.sh --audit <workspace-id>

Close the tab a pane lives in and prove the close. The tab id is resolved from
the pane id, so the pane id the orchestrator holds is enough.

  <pane-id>              resolve this pane's tab, close the tab, and prove the
                         tab is gone from `herdr tab list --workspace <ID>`
  --audit <workspace-id> print the workspace's tabs with their agent state and
                         close nothing - the leak check
  -h, --help             show this text

The remaining tabs are printed on stdout after the close, one line each, with
the agent state; the verdict is on stderr.

Exit codes:
  0  the close is proved: the tab is gone from the workspace's listing
  2  usage error
  3  the pane id resolves to no tab
  4  the close was refused, the tab is still listed, or a Herdr call failed
EOF
}

die_usage() {
  printf 'ERROR: %s\n' "$*" >&2
  usage >&2
  exit 2
}

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 4
}

while (($#)); do
  case "$1" in
    --audit)
      AUDIT=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    --)
      shift
      while (($#)); do
        POSITIONAL+=("$1")
        shift
      done
      ;;
    -*)
      printf 'ERROR: unknown option: %s\n' "$1" >&2
      exit 2
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

((${#POSITIONAL[@]} == 1)) || die_usage "expected exactly one id"
TARGET=${POSITIONAL[0]}

if ((AUDIT)); then
  case "$TARGET" in
    *:p*) die_usage "--audit takes a workspace id, not a pane id: $TARGET" ;;
  esac
fi

command -v herdr >/dev/null 2>&1 || fail "herdr is required"
command -v python3 >/dev/null 2>&1 || fail "python3 is required"

# Print "TAB_ID<TAB>WORKSPACE_ID" for a pane. Exit 3 when the pane resolves to
# no tab: herdr does not know the pane, or the pane carries no tab_id or
# workspace_id. Exit 4 on a Herdr failure or a malformed response.
parse_pane() {
  python3 -c '
import json
import sys

pane_id = sys.argv[1]
raw = sys.stdin.read()
try:
    doc = json.loads(raw)
except Exception as exc:
    print("invalid herdr JSON: %s" % exc, file=sys.stderr)
    raise SystemExit(4)

if not isinstance(doc, dict):
    print("invalid herdr response: not an object", file=sys.stderr)
    raise SystemExit(4)

error = doc.get("error")
if error is not None:
    code = error.get("code") if isinstance(error, dict) else None
    if code == "pane_not_found":
        print("pane %s resolves to nothing: herdr does not know it" % pane_id,
              file=sys.stderr)
        raise SystemExit(3)
    if isinstance(error, dict):
        message = error.get("message") or error.get("code") or json.dumps(error)
    else:
        message = str(error)
    print("herdr error: %s" % message, file=sys.stderr)
    raise SystemExit(4)

result = doc.get("result")
pane = result.get("pane") if isinstance(result, dict) else None
if not isinstance(pane, dict):
    print("invalid herdr response: no pane object", file=sys.stderr)
    raise SystemExit(4)

tab_id = pane.get("tab_id")
workspace_id = pane.get("workspace_id")
if not isinstance(tab_id, str) or not tab_id:
    print("pane %s resolves to no tab: it carries no tab id" % pane_id,
          file=sys.stderr)
    raise SystemExit(3)
if not isinstance(workspace_id, str) or not workspace_id:
    print("pane %s resolves to no tab: it carries no workspace id" % pane_id,
          file=sys.stderr)
    raise SystemExit(3)

print(tab_id + "\t" + workspace_id)
' "$1"
}

# Print the workspace's tabs, one line each, with agent state and pane count.
# $1 is the workspace id, $2 a tab id that must be absent ("" lists only).
# Exit 0: listed, and $2 is absent (or empty). Exit 1: $2 is still listed.
# Exit 4: Herdr or parse failure. Exit 5: herdr has no such workspace.
summarize_tabs() {
  python3 -c '
import json
import sys

workspace = sys.argv[1]
expect_gone = sys.argv[2]
raw = sys.stdin.read()
try:
    doc = json.loads(raw)
except Exception as exc:
    print("invalid herdr JSON: %s" % exc, file=sys.stderr)
    raise SystemExit(4)

if not isinstance(doc, dict):
    print("invalid herdr response: not an object", file=sys.stderr)
    raise SystemExit(4)

error = doc.get("error")
if error is not None:
    code = error.get("code") if isinstance(error, dict) else None
    if code == "workspace_not_found":
        print("herdr has no workspace %s" % workspace, file=sys.stderr)
        raise SystemExit(5)
    if isinstance(error, dict):
        message = error.get("message") or error.get("code") or json.dumps(error)
    else:
        message = str(error)
    print("herdr error: %s" % message, file=sys.stderr)
    raise SystemExit(4)

result = doc.get("result")
tabs = result.get("tabs") if isinstance(result, dict) else None
if not isinstance(tabs, list):
    print("invalid herdr response: no tabs list", file=sys.stderr)
    raise SystemExit(4)

listed = False
rows = []
for tab in tabs:
    if not isinstance(tab, dict) or not isinstance(tab.get("tab_id"), str):
        print("invalid herdr response: a tab has no tab_id", file=sys.stderr)
        raise SystemExit(4)
    tab_id = tab["tab_id"]
    number = tab.get("number")
    agent = tab.get("agent_status") or "unknown"
    panes = tab.get("pane_count")
    label = json.dumps(tab.get("label") or "")
    if tab_id == expect_gone:
        listed = True
    rows.append("  %-9s #%-2s %-8s panes=%s  label=%s"
                % (tab_id, number, agent, panes, label))

if rows:
    print("tabs in workspace %s (%d):" % (workspace, len(rows)))
    for row in rows:
        print(row)
else:
    print("tabs in workspace %s: none" % workspace)

raise SystemExit(1 if expect_gone and listed else 0)
' "$1" "$2"
}

# Ask herdr for the workspace's tabs and print the listing.
list_tabs() {
  local raw rc
  : >"$ERR_FILE"
  # SIGKILL keeps a hung client from hanging the closer; GNU timeout reports
  # 137 for a SIGKILL timeout, not the usual 124.
  raw=$(timeout --signal=KILL "$CALL_TIMEOUT" herdr tab list --workspace "$1" 2>"$ERR_FILE")
  rc=$?
  if ((rc == 124 || rc == 137)); then
    printf 'ERROR: herdr tab list for workspace %s exceeded the %ss call bound\n' \
      "$1" "$CALL_TIMEOUT" >&2
    return 4
  fi
  if ((rc == 0)); then
    # stderr is not part of the response: a warning passes through, never into
    # the JSON the parser reads.
    if [[ -s "$ERR_FILE" ]]; then
      cat "$ERR_FILE" >&2
    fi
  elif [[ -s "$ERR_FILE" ]]; then
    # herdr carries its error envelope on stderr; hand it to the parser apart
    # from stdout.
    raw=$(<"$ERR_FILE")
  fi
  printf '%s' "$raw" | summarize_tabs "$1" "$2"
}

if ((AUDIT)); then
  # The audit names the workspace, not a pane: a round's pane is gone with its
  # tab, and the leak check runs on the workspace the orchestrator created.
  list_tabs "$TARGET" ""
  (( $? == 0 )) || exit 4
  exit 0
fi

PANE=$TARGET
: >"$ERR_FILE"
# SIGKILL keeps a hung client from hanging the closer; GNU timeout reports 137
# for a SIGKILL timeout, not the usual 124.
raw=$(timeout --signal=KILL "$CALL_TIMEOUT" herdr pane get "$PANE" 2>"$ERR_FILE")
call_rc=$?
if ((call_rc == 124 || call_rc == 137)); then
  fail "herdr pane get for pane $PANE exceeded the ${CALL_TIMEOUT}s call bound"
fi
if ((call_rc == 0)); then
  # stderr is not part of the response: a warning passes through, never into
  # the JSON the parser reads.
  if [[ -s "$ERR_FILE" ]]; then
    cat "$ERR_FILE" >&2
  fi
elif [[ -s "$ERR_FILE" ]]; then
  # herdr carries its error envelope on stderr; hand it to the parser apart
  # from stdout.
  raw=$(<"$ERR_FILE")
fi
state=$(printf '%s' "$raw" | parse_pane "$PANE")
parse_rc=$?
case "$parse_rc" in
  0) ;;
  3) exit 3 ;;
  *) exit 4 ;;
esac
TAB_ID=${state%%$'\t'*}
WORKSPACE_ID=${state#*$'\t'}

printf 'pane %s is in tab %s (workspace %s)\n' "$PANE" "$TAB_ID" "$WORKSPACE_ID"

: >"$ERR_FILE"
close_out=$(timeout --signal=KILL "$CALL_TIMEOUT" herdr tab close "$TAB_ID" 2>"$ERR_FILE")
close_rc=$?
if ((close_rc == 124 || close_rc == 137)); then
  fail "herdr tab close for tab $TAB_ID exceeded the ${CALL_TIMEOUT}s call bound"
fi
if ((close_rc != 0)); then
  if [[ -s "$ERR_FILE" ]]; then
    close_out=$(<"$ERR_FILE")
  fi
  case "$close_out" in
    *tab_not_found*)
      printf 'ERROR: herdr answered tab_not_found for tab %s: the tab was not closed; the listing below is the evidence\n' \
        "$TAB_ID" >&2
      ;;
    *)
      printf 'ERROR: herdr tab close %s failed: %s\n' "$TAB_ID" "$close_out" >&2
      ;;
  esac
  list_tabs "$WORKSPACE_ID" ""
  exit 4
fi
if [[ -s "$ERR_FILE" ]]; then
  cat "$ERR_FILE" >&2
fi

# The close is done when the listing proves it. Print the listing either way:
# it is the evidence, and it names the tabs that are still there.
list_tabs "$WORKSPACE_ID" "$TAB_ID"
list_rc=$?
case "$list_rc" in
  0)
    printf 'CLOSED: tab %s is gone from workspace %s (pane %s)\n' \
      "$TAB_ID" "$WORKSPACE_ID" "$PANE" >&2
    exit 0
    ;;
  1)
    printf 'ERROR: tab %s is still listed in workspace %s: the close is not proved\n' \
      "$TAB_ID" "$WORKSPACE_ID" >&2
    exit 4
    ;;
  5)
    printf 'CLOSED: tab %s was the last tab of workspace %s, which herdr closed with it (pane %s)\n' \
      "$TAB_ID" "$WORKSPACE_ID" "$PANE" >&2
    exit 0
    ;;
  *)
    printf 'ERROR: the listing of workspace %s failed: the close of tab %s is not proved\n' \
      "$WORKSPACE_ID" "$TAB_ID" >&2
    exit 4
    ;;
esac
