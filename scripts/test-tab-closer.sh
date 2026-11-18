#!/usr/bin/env bash
# Agent-runnable tests for scripts/herdr-close-pane.sh.
#
# The tests put a fake `herdr` first on PATH. It serves JSON fixtures for
# `pane get` and `tab list`, records the arguments of every call, and drops a
# tab from the listing once `tab close` records it. No real Herdr session is
# touched.
#
# Every case runs under a wall-clock bound, so a hang fails the test instead of
# hanging it.
#
# Dependencies: bash, coreutils (mktemp, timeout).
set -uo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CLOSER="$HERE/herdr-close-pane.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/herdr-close-pane-test.XXXXXX")
CASE_TIMEOUT=${CASE_TIMEOUT:-30}
if ! [[ "$CASE_TIMEOUT" =~ ^[1-9][0-9]*$ ]]; then
  CASE_TIMEOUT=30
fi
trap 'rm -rf "$TMP"' EXIT

BIN="$TMP/bin"
mkdir -p "$BIN"
cat >"$BIN/herdr" <<'FAKE'
#!/usr/bin/env bash
set -u
state=${FAKE_STATE_DIR:?FAKE_STATE_DIR not set}
cmd=${1:-}
shift || true

if [[ -n ${FAKE_WARN:-} ]]; then
  printf 'herdr: warning: %s\n' "$FAKE_WARN" >&2
fi
if [[ ${FAKE_HANG:-} == "$cmd" ]]; then
  exec sleep 300
fi
if [[ "$cmd" == tab && ${FAKE_HANG:-} == "${1:-}" ]]; then
  exec sleep 300
fi

list_missing() {
  printf '{"error":{"code":"workspace_not_found","message":"workspace %s not found"}}\n' "$1" >&2
  exit 1
}

case "$cmd" in
  pane)
    sub=${1:-}
    shift || true
    case "$sub" in
      get)
        printf '%s\n' "$@" >"$state/pane-args"
        if [[ -f "$state/pane.json" ]]; then
          cat "$state/pane.json"
          exit 0
        fi
        printf '{"error":{"code":"pane_not_found","message":"pane %s not found"}}\n' "${1:-}" >&2
        exit 1
        ;;
      *)
        printf 'fake herdr: unsupported pane command %s\n' "$sub" >&2
        exit 2
        ;;
    esac
    ;;
  tab)
    sub=${1:-}
    shift || true
    case "$sub" in
      close)
        printf '%s\n' "$@" >"$state/close-args"
        tab=${1:-}
        if [[ ${FAKE_CLOSE_FAIL:-} == tab_not_found ]]; then
          printf '{"error":{"code":"tab_not_found","message":"tab %s not found"}}\n' "$tab" >&2
          exit 1
        fi
        if [[ ${FAKE_CLOSE_FAIL:-} == internal_error ]]; then
          printf '{"error":{"code":"internal_error","message":"fake close failure"}}\n' >&2
          exit 1
        fi
        printf '%s\n' "$tab" >"$state/closed"
        printf '{"id":"cli:tab:close","result":{"type":"tab_close"}}\n'
        ;;
      list)
        printf '%s\n' "$@" >"$state/list-args"
        workspace=""
        while (($#)); do
          case "$1" in
            --workspace)
              workspace=${2:-}
              shift 2
              ;;
            *) shift ;;
          esac
        done
        if [[ ${FAKE_LIST_FAIL:-} == internal_error ]]; then
          printf '{"error":{"code":"internal_error","message":"fake list failure"}}\n' >&2
          exit 1
        fi
        if [[ -f "$state/closed" ]]; then
          if [[ -f "$state/tabs-after.json" ]]; then
            cat "$state/tabs-after.json"
            exit 0
          fi
          if [[ -f "$state/tabs-after.gone" ]]; then
            list_missing "$workspace"
          fi
        fi
        if [[ -f "$state/tabs.json" ]]; then
          cat "$state/tabs.json"
          exit 0
        fi
        list_missing "$workspace"
        ;;
      *)
        printf 'fake herdr: unsupported tab command %s\n' "$sub" >&2
        exit 2
        ;;
    esac
    ;;
  *)
    printf 'fake herdr: unsupported command %s\n' "$cmd" >&2
    exit 2
    ;;
esac
FAKE
chmod +x "$BIN/herdr"
export PATH="$BIN:$PATH"

PASS=0
FAIL=0
CASE=0
RC=0

ok() { PASS=$((PASS + 1)); }
bad() {
  FAIL=$((FAIL + 1))
  printf 'FAIL: %s\n' "$*" >&2
}

new_case() {
  CASE=$((CASE + 1))
  FAKE_STATE_DIR="$TMP/case$CASE"
  mkdir -p "$FAKE_STATE_DIR"
  export FAKE_STATE_DIR
  unset FAKE_CLOSE_FAIL FAKE_LIST_FAIL FAKE_WARN FAKE_HANG HERDR_CALL_TIMEOUT
  STDOUT="$FAKE_STATE_DIR/stdout"
  STDERR="$FAKE_STATE_DIR/stderr"
  RC=0
}

fixture() {
  printf '%s\n' "$2" >"$FAKE_STATE_DIR/$1"
}

run_closer() {
  timeout "$CASE_TIMEOUT" bash "$CLOSER" "$@" >"$STDOUT" 2>"$STDERR"
  RC=$?
  if ((RC == 124)); then
    bad "case $CASE exceeded ${CASE_TIMEOUT}s wall-clock bound"
  fi
}

expect_rc() {
  local want=$1 label=$2
  if [[ "$RC" == "$want" ]]; then
    ok
  else
    bad "$label: exit $RC, want $want; stderr: $(<"$STDERR")"
  fi
}

expect_contains() {
  local file=$1 needle=$2 label=$3 text
  text=$(<"$file")
  if [[ "$text" == *"$needle"* ]]; then
    ok
  else
    bad "$label: '$needle' not found in $file"
  fi
}

expect_not_contains() {
  local file=$1 needle=$2 label=$3 text
  text=$(<"$file")
  if [[ "$text" != *"$needle"* ]]; then
    ok
  else
    bad "$label: '$needle' unexpectedly found in $file"
  fi
}

expect_file() {
  local file=$1 want=$2 label=$3 text
  text=$(<"$file")
  if [[ "$text" == "$want" ]]; then
    ok
  else
    bad "$label: got '$text', want '$want'"
  fi
}

expect_missing() {
  local file=$1 label=$2
  if [[ ! -e "$file" ]]; then
    ok
  else
    bad "$label: $file should not exist"
  fi
}

# The pane id the orchestrator holds is w0:p7; its tab is w0:t2. The two ids
# differ, so a close that passes the pane id through cannot pass these cases.
pane_fixture() {
  fixture pane.json '{"id":"cli:pane:get","result":{"pane":{"agent_status":"working","pane_id":"w0:p7","tab_id":"w0:t2","workspace_id":"w0"},"type":"pane_info"}}'
}

tabs_fixture() {
  fixture tabs.json '{"id":"cli:tab:list","result":{"tabs":[{"agent_status":"done","focused":false,"label":"1","number":1,"pane_count":1,"tab_id":"w0:t1","workspace_id":"w0"},{"agent_status":"working","focused":false,"label":"build-tab-closer","number":2,"pane_count":1,"tab_id":"w0:t2","workspace_id":"w0"}],"type":"tab_list"}}'
}

tabs_after_fixture() {
  fixture tabs-after.json '{"id":"cli:tab:list","result":{"tabs":[{"agent_status":"done","focused":false,"label":"1","number":1,"pane_count":1,"tab_id":"w0:t1","workspace_id":"w0"}],"type":"tab_list"}}'
}

# 1. The happy path: resolve the tab from the pane, close that tab, prove it
# is gone, and list what remains.
new_case
pane_fixture
tabs_fixture
tabs_after_fixture
run_closer w0:p7
expect_rc 0 "happy path"
expect_file "$FAKE_STATE_DIR/pane-args" 'w0:p7' "resolution asks herdr for the pane id"
expect_file "$FAKE_STATE_DIR/close-args" 'w0:t2' "close uses the resolved tab id, not the pane id"
expect_contains "$STDOUT" 'pane w0:p7 is in tab w0:t2 (workspace w0)' "happy path prints the resolution"
expect_contains "$STDERR" 'CLOSED: tab w0:t2 is gone from workspace w0' "happy path says the close is proved"
expect_contains "$STDOUT" 'tabs in workspace w0 (1):' "happy path lists the remaining tabs"
expect_contains "$STDOUT" 'w0:t1' "happy path lists the tab that remains"
expect_contains "$STDOUT" 'done' "listing carries the agent state"
expect_not_contains "$STDOUT" $'\n  w0:t2' "the closed tab is gone from the listing"
expect_file "$FAKE_STATE_DIR/list-args" $'--workspace\nw0' "the listing asks for the resolved workspace"

# 2. The tab is listed after the close: the close is not proved.
new_case
pane_fixture
tabs_fixture
run_closer w0:p7
expect_rc 4 "tab still listed"
expect_contains "$STDERR" 'still listed' "still listed is named"
expect_not_contains "$STDERR" 'CLOSED:' "still listed never reports a close"
expect_contains "$STDOUT" 'w0:t2' "still listed prints the evidence"
expect_contains "$STDOUT" 'tabs in workspace w0 (2):' "still listed counts both tabs"

# 3. herdr refuses the close with tab_not_found: the tab was not closed.
new_case
pane_fixture
tabs_fixture
export FAKE_CLOSE_FAIL=tab_not_found
run_closer w0:p7
expect_rc 4 "tab_not_found"
expect_contains "$STDERR" 'tab_not_found' "tab_not_found is surfaced"
expect_contains "$STDERR" 'the tab was not closed' "tab_not_found is not read as already closed"
expect_not_contains "$STDERR" 'CLOSED:' "tab_not_found never reports a close"
expect_contains "$STDOUT" 'w0:t2' "tab_not_found prints the listing as evidence"

# 4. Another Herdr failure on the close is an error, not a close.
new_case
pane_fixture
tabs_fixture
export FAKE_CLOSE_FAIL=internal_error
run_closer w0:p7
expect_rc 4 "close failure"
expect_contains "$STDERR" 'herdr tab close w0:t2 failed' "close failure names the call"
expect_contains "$STDERR" 'fake close failure' "close failure carries the herdr message"

# 5. The pane id herdr does not know resolves to nothing.
new_case
run_closer w0:p9
expect_rc 3 "pane not found"
expect_contains "$STDERR" 'pane w0:p9 resolves to nothing' "pane not found names the pane"
expect_missing "$FAKE_STATE_DIR/close-args" "pane not found closes nothing"
expect_missing "$FAKE_STATE_DIR/list-args" "pane not found lists nothing"

# 6. A pane that carries no tab id resolves to nothing.
new_case
fixture pane.json '{"result":{"pane":{"pane_id":"w0:p7","workspace_id":"w0"}}}'
run_closer w0:p7
expect_rc 3 "pane without tab id"
expect_contains "$STDERR" 'carries no tab id' "pane without tab id is named"
expect_missing "$FAKE_STATE_DIR/close-args" "pane without tab id closes nothing"

# 7. A pane that carries no workspace id cannot be proved, so it resolves to
# nothing.
new_case
fixture pane.json '{"result":{"pane":{"pane_id":"w0:p7","tab_id":"w0:t2"}}}'
run_closer w0:p7
expect_rc 3 "pane without workspace id"
expect_contains "$STDERR" 'carries no workspace id' "pane without workspace id is named"
expect_missing "$FAKE_STATE_DIR/close-args" "pane without workspace id closes nothing"

# 8. A malformed pane response is a Herdr failure.
new_case
fixture pane.json 'not json at all'
run_closer w0:p7
expect_rc 4 "malformed pane response"
expect_contains "$STDERR" 'invalid herdr JSON' "malformed pane response is named"

# 9. The close took the last tab, and herdr closed the workspace with it: the
# listing proves the tab is gone.
new_case
pane_fixture
fixture tabs.json '{"result":{"tabs":[{"agent_status":"working","label":"build-tab-closer","number":2,"pane_count":1,"tab_id":"w0:t2","workspace_id":"w0"}],"type":"tab_list"}}'
fixture tabs-after.gone ''
run_closer w0:p7
expect_rc 0 "last tab takes the workspace"
expect_contains "$STDERR" 'was the last tab of workspace w0' "last tab names the workspace it took"
expect_not_contains "$STDERR" 'ERROR:' "last tab is not an error"

# 10. The listing itself fails: the close is not proved.
new_case
pane_fixture
tabs_fixture
export FAKE_LIST_FAIL=internal_error
run_closer w0:p7
expect_rc 4 "listing failure"
expect_contains "$STDERR" 'is not proved' "listing failure names the missing proof"
expect_not_contains "$STDERR" 'CLOSED:' "listing failure never reports a close"

# 11. The audit lists the workspace's tabs with their state and closes nothing.
new_case
fixture tabs.json '{"result":{"tabs":[{"agent_status":"done","label":"build-tab-closer","number":1,"pane_count":1,"tab_id":"w0:t1","workspace_id":"w0"},{"agent_status":"idle","label":"review-tab-closer-code","number":2,"pane_count":1,"tab_id":"w0:t2","workspace_id":"w0"},{"agent_status":"working","label":"3","number":3,"pane_count":2,"tab_id":"w0:t3","workspace_id":"w0"}],"type":"tab_list"}}'
run_closer --audit w0
expect_rc 0 "audit"
expect_contains "$STDOUT" 'tabs in workspace w0 (3):' "audit counts the tabs"
expect_contains "$STDOUT" 'done' "audit shows a finished agent"
expect_contains "$STDOUT" 'idle' "audit shows an idle agent"
expect_contains "$STDOUT" 'working' "audit shows a working agent"
expect_contains "$STDOUT" 'build-tab-closer' "audit names the tab label"
expect_contains "$STDOUT" 'panes=2' "audit carries the pane count"
expect_missing "$FAKE_STATE_DIR/pane-args" "audit resolves no pane"
expect_missing "$FAKE_STATE_DIR/close-args" "audit closes nothing"
expect_file "$FAKE_STATE_DIR/list-args" $'--workspace\nw0' "audit lists the named workspace"

# 12. The audit of a workspace herdr does not know fails.
new_case
run_closer --audit w9
expect_rc 4 "audit of a missing workspace"
expect_contains "$STDERR" 'herdr has no workspace w9' "audit of a missing workspace is named"

# 13. An audit with no workspace id is a usage error.
new_case
run_closer --audit
expect_rc 2 "audit without an id"
expect_contains "$STDERR" 'Usage:' "audit without an id prints usage"
expect_missing "$FAKE_STATE_DIR/list-args" "audit without an id calls nothing"

# 14-17. Usage errors exit 2 before any Herdr call.
new_case
run_closer
expect_rc 2 "missing pane id"
expect_contains "$STDERR" 'Usage:' "missing pane id prints usage"

new_case
run_closer w0:p7 w0:t2
expect_rc 2 "two ids"
expect_contains "$STDERR" 'expected exactly one id' "two ids are refused"

new_case
run_closer --bogus
expect_rc 2 "unknown option"
expect_contains "$STDERR" 'unknown option' "unknown option is named"
expect_missing "$FAKE_STATE_DIR/pane-args" "unknown option calls nothing"

new_case
run_closer --help
expect_rc 0 "help"
expect_contains "$STDOUT" 'Usage:' "help prints usage"
expect_contains "$STDOUT" '--audit' "help names the audit flag"
expect_missing "$FAKE_STATE_DIR/pane-args" "help calls nothing"

# 18. A pane id after `--` is a target, not an option.
new_case
pane_fixture
tabs_fixture
tabs_after_fixture
run_closer -- w0:p7
expect_rc 0 "target after --"
expect_file "$FAKE_STATE_DIR/close-args" 'w0:t2' "target after -- reaches the close"

# 19. An empty workspace lists as none.
new_case
fixture tabs.json '{"result":{"tabs":[],"type":"tab_list"}}'
run_closer --audit w0
expect_rc 0 "empty audit"
expect_contains "$STDOUT" 'tabs in workspace w0: none' "empty audit says none"

# 20. A herdr warning on stderr passes through without breaking the JSON parse.
new_case
pane_fixture
tabs_fixture
tabs_after_fixture
export FAKE_WARN='deprecated option: --workspace'
run_closer w0:p7
expect_rc 0 "warning on stderr"
expect_contains "$STDERR" 'CLOSED: tab w0:t2 is gone from workspace w0' "warning does not break the proof"
expect_contains "$STDERR" 'deprecated option' "the warning is reported separately"
expect_contains "$STDOUT" 'tabs in workspace w0 (1):' "warning does not break the listing"

# 21. An audit given a pane id is a usage error, not a Herdr failure.
new_case
run_closer --audit w0:p7
expect_rc 2 "audit of a pane id"
expect_contains "$STDERR" 'workspace id, not a pane id' "audit of a pane id names the mistake"
expect_contains "$STDERR" 'Usage:' "audit of a pane id prints usage"
expect_missing "$FAKE_STATE_DIR/list-args" "audit of a pane id calls nothing"

# 22. A hung pane get is bounded: the closer fails instead of hanging.
new_case
export FAKE_HANG=pane
export HERDR_CALL_TIMEOUT=1
run_closer w0:p7
expect_rc 4 "hung pane get"
expect_contains "$STDERR" 'herdr pane get for pane w0:p7 exceeded the 1s call bound' "hung pane get names the bound"
expect_missing "$FAKE_STATE_DIR/close-args" "hung pane get closes nothing"

# 23. A hung tab list is bounded too.
new_case
export FAKE_HANG=list
export HERDR_CALL_TIMEOUT=1
run_closer --audit w0
expect_rc 4 "hung tab list"
expect_contains "$STDERR" 'herdr tab list for workspace w0 exceeded the 1s call bound' "hung tab list names the bound"

# 24. A hung tab close is bounded too.
new_case
pane_fixture
tabs_fixture
export FAKE_HANG=close
export HERDR_CALL_TIMEOUT=1
run_closer w0:p7
expect_rc 4 "hung tab close"
expect_contains "$STDERR" 'herdr tab close for tab w0:t2 exceeded the 1s call bound' "hung tab close names the bound"
expect_missing "$FAKE_STATE_DIR/list-args" "hung tab close proves nothing"

printf 'herdr-close-pane: %d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
