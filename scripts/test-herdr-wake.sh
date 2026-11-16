#!/usr/bin/env bash
# Agent-runnable tests for scripts/herdr-wake.sh.
#
# The tests put a fake `herdr` first on PATH. It serves JSON fixtures for
# `agent get`, records `agent prompt` and `pane read` arguments, and prints
# pane output. No real Herdr session is touched.
#
# Every case runs under a wall-clock bound, so a missing script timeout fails
# the test instead of hanging it.
#
# Dependencies: bash, python3, coreutils (mktemp, timeout).
set -uo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WAKE="$HERE/herdr-wake.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/herdr-wake-test.XXXXXX")
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

# Pick an exact-count fixture first, then a phase sequence with its last line
# repeating, then a single phase fixture.
pick_fixture() {
  local phase=$1 count=$2 candidate index_file index line
  local -a fixtures
  for candidate in "$state/$phase.$count.json" "$state/$phase.json"; do
    if [[ -f $candidate ]]; then
      cat "$candidate"
      return 0
    fi
  done
  if [[ -f "$state/$phase.seq" ]]; then
    mapfile -t fixtures <"$state/$phase.seq"
    ((${#fixtures[@]} > 0)) || return 1
    index_file="$state/$phase.seq-index"
    index=0
    [[ -f $index_file ]] && index=$(cat "$index_file")
    line=$((index + 1))
    ((line > ${#fixtures[@]})) && line=${#fixtures[@]}
    printf '%s\n' "${fixtures[$((line - 1))]}"
    printf '%s\n' "$((index + 1))" >"$index_file"
    return 0
  fi
  return 1
}

case "$cmd" in
  agent)
    sub=${1:-}
    shift || true
    case "$sub" in
      get)
        if [[ ${FAKE_GET_HANG:-0} == 1 ]]; then
          exec sleep 60
        fi
        if [[ -n ${FAKE_GET_ERROR:-} ]]; then
          printf '{"error":{"code":"%s","message":"fake herdr failure"}}\n' "$FAKE_GET_ERROR" >&2
          exit 1
        fi
        not_found_budget=${FAKE_NOT_FOUND_COUNT:-0}
        if ((not_found_budget > 0)); then
          used_file="$state/not-found-used"
          used=0
          [[ -f $used_file ]] && used=$(cat "$used_file")
          if ((used < not_found_budget)); then
            printf '%s\n' "$((used + 1))" >"$used_file"
            printf '%s\n' '{"error":{"code":"agent_not_found","message":"agent target not found"}}' >&2
            exit 1
          fi
        fi
        count_file="$state/get-count"
        count=0
        [[ -f $count_file ]] && count=$(cat "$count_file")
        count=$((count + 1))
        printf '%s\n' "$count" >"$count_file"
        phase=before
        [[ -f "$state/phase" ]] && phase=$(cat "$state/phase")
        if pick_fixture "$phase" "$count"; then
          exit 0
        fi
        printf '%s\n' '{"error":{"code":"missing_fixture","message":"fake herdr has no state fixture"}}' >&2
        exit 1
        ;;
      prompt)
        shift || true # target
        text=${1:-}
        printf '%s' "$text" >"$state/prompt.txt"
        if [[ ${FAKE_PROMPT_HANG:-0} == 1 ]]; then
          exec sleep 60
        fi
        if [[ ${FAKE_PROMPT_FAIL:-0} == 1 ]]; then
          printf '%s\n' '{"error":{"code":"agent_blocked","message":"fake prompt blocked"}}' >&2
          exit 1
        fi
        printf 'after\n' >"$state/phase"
        printf '%s\n' '{"result":{"agent_prompted":{"pane_id":"w1:p1"}}}'
        ;;
      *)
        printf 'fake herdr: unsupported agent command %s\n' "$sub" >&2
        exit 2
        ;;
    esac
    ;;
  pane)
    sub=${1:-}
    shift || true
    case "$sub" in
      read)
        printf '%s\n' "$@" >"$state/pane-args"
        lines=""
        while (($#)); do
          case "$1" in
            --lines)
              lines=${2:-}
              shift 2
              ;;
            *)
              shift
              ;;
          esac
        done
        printf '%s\n' "$lines" >"$state/pane-lines"
        if [[ ${FAKE_PANE_HANG:-0} == 1 ]]; then
          exec sleep 60
        fi
        if [[ ${FAKE_PANE_FAIL:-0} == 1 ]]; then
          printf 'fake herdr pane read: simulated failure\n' >&2
          exit 1
        fi
        printf '%s\n' "${FAKE_PANE_TEXT:-final pane output}"
        ;;
      *)
        printf 'fake herdr: unsupported pane command %s\n' "$sub" >&2
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
ELAPSED=0

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
  unset FAKE_GET_ERROR FAKE_PROMPT_FAIL FAKE_PANE_TEXT FAKE_PANE_FAIL
  unset FAKE_PANE_HANG FAKE_NOT_FOUND_COUNT FAKE_GET_HANG FAKE_PROMPT_HANG
  STDOUT="$FAKE_STATE_DIR/stdout"
  STDERR="$FAKE_STATE_DIR/stderr"
  RC=0
  ELAPSED=0
}

fixture() {
  printf '%s\n' "$2" >"$FAKE_STATE_DIR/$1"
}

run_wake_bounded() {
  local bound=$1
  shift
  local start=$SECONDS
  timeout "$bound" bash "$WAKE" "$@" >"$STDOUT" 2>"$STDERR"
  RC=$?
  ELAPSED=$((SECONDS - start))
  if ((RC == 124)); then
    bad "case $CASE exceeded ${bound}s wall-clock bound (script timeout missing or broken)"
  fi
}

run_wake() {
  run_wake_bounded "$CASE_TIMEOUT" "$@"
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

expect_elapsed_between() {
  local min=$1 max=$2 label=$3
  if ((ELAPSED >= min && ELAPSED <= max)); then
    ok
  else
    bad "$label: elapsed ${ELAPSED}s, want ${min}-${max}s"
  fi
}

# 1. A settled turn whose completion_seq advances wakes immediately.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
export FAKE_PANE_TEXT='agent summary: unit done'
run_wake w1:p1 'do the thing' --timeout 5 --interval 1
expect_rc 0 "advance"
expect_contains "$STDOUT" 'agent summary: unit done' "advance prints pane output"
expect_contains "$STDERR" 'completion_seq 7 -> 8' "advance reports the transition"
expect_file "$FAKE_STATE_DIR/prompt.txt" 'do the thing' "advance sends prompt as one argument"
expect_file "$FAKE_STATE_DIR/pane-lines" '40' "advance uses the default line count"
expect_contains "$FAKE_STATE_DIR/pane-args" 'recent-unwrapped' "advance reads unwrapped pane output"

# 2. A null baseline means no earlier completion: any later seq is a wake.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle"}}}'
fixture after.json '{"result":{"agent":{"agent_status":"done","completion_seq":1}}}'
run_wake w1:p1 'first turn' --timeout 5 --interval 1
expect_rc 0 "null baseline"
expect_contains "$STDERR" 'completion_seq none -> 1' "null baseline reports the transition"

# 3. An unchanged completion_seq times out with a distinct exit and pane output.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
export FAKE_PANE_TEXT='still waiting in the pane'
run_wake w1:p1 'slow turn' --timeout 3 --interval 1
expect_rc 3 "timeout"
expect_contains "$STDERR" 'TIMEOUT:' "timeout is labeled"
expect_contains "$STDERR" 'did not advance within the 3s command bound' "timeout names the command bound"
expect_contains "$STDERR" 'no command-bound budget left' "timeout reports the missing pane output"
expect_file "$STDOUT" '' "timeout leaves stdout empty when the bound is spent"
expect_elapsed_between 2 5 "timeout is bounded near --timeout"

# 4. Blocked wins over completion: a blocked state with an advanced seq exits 2.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"blocked","completion_seq":8}}}'
export FAKE_PANE_TEXT='blocked: needs approval'
run_wake w1:p1 'needs approval' --timeout 5 --interval 1
expect_rc 2 "blocked wins over completion"
expect_contains "$STDERR" 'BLOCKED:' "blocked wins is labeled"
expect_contains "$STDOUT" 'blocked: needs approval' "blocked wins prints pane output"
expect_not_contains "$STDERR" 'WAKE:' "blocked wins does not report a wake"

# 5. A blocked pre-dispatch state exits blocked without sending the prompt.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"blocked","completion_seq":3}}}'
export FAKE_PANE_TEXT='approval needed'
run_wake w1:p1 'must not send' --timeout 5 --interval 1
expect_rc 2 "blocked before dispatch"
expect_contains "$STDERR" 'blocked before dispatch' "blocked before dispatch is labeled"
expect_contains "$STDOUT" 'approval needed' "blocked before dispatch prints pane output"
expect_missing "$FAKE_STATE_DIR/prompt.txt" "blocked before dispatch sends no prompt"

# 6. Work already in flight settles first, so its seq is the baseline.
new_case
fixture before.seq $'{"result":{"agent":{"agent_status":"working"}}}\n{"result":{"agent":{"agent_status":"idle","completion_seq":4}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":5}}}'
run_wake w1:p1 'next turn' --timeout 5 --interval 1
expect_rc 0 "pre-settle"
expect_contains "$STDERR" 'completion_seq 4 -> 5' "pre-settle uses the settled baseline"

# 7. Two consecutive settled reads are required before the baseline.
new_case
fixture before.seq $'{"result":{"agent":{"agent_status":"idle","completion_seq":4}}}\n{"result":{"agent":{"agent_status":"working"}}}\n{"result":{"agent":{"agent_status":"idle","completion_seq":5}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":6}}}'
run_wake w1:p1 'two settled reads' --timeout 5 --interval 1
expect_rc 0 "two settled reads"
expect_contains "$STDERR" 'completion_seq 5 -> 6' "baseline comes from two settled reads"

# 8. A rejected prompt that Herdr reports as blocked exits blocked.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
export FAKE_PROMPT_FAIL=1
run_wake w1:p1 'rejected' --timeout 5 --interval 1
expect_rc 2 "prompt rejected as blocked"
expect_contains "$STDERR" 'BLOCKED:' "prompt rejection is labeled"

# 9. A Herdr failure that is not agent_not_found is an error, not a wake.
new_case
export FAKE_GET_ERROR=internal_error
run_wake w1:p1 'no agent' --timeout 5 --interval 1
expect_rc 4 "herdr failure"
expect_contains "$STDERR" 'herdr error' "herdr failure surfaces the error"

# 10. agent_not_found is not-ready: keep polling, then dispatch and wake.
new_case
export FAKE_NOT_FOUND_COUNT=3
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
run_wake w1:p1 'wait for agent' --timeout 10 --interval 1
expect_rc 0 "agent_not_found retries until ready"
expect_contains "$STDERR" 'completion_seq 7 -> 8' "readiness retry reaches a wake"
expect_file "$FAKE_STATE_DIR/prompt.txt" 'wait for agent' "readiness retry sends the prompt"

# 11. agent_not_found with no readiness expires as a timeout, not exit 4.
new_case
export FAKE_NOT_FOUND_COUNT=999
run_wake w1:p1 'never ready' --timeout 1 --interval 1
expect_rc 3 "agent_not_found until timeout"
expect_contains "$STDERR" 'did not settle' "agent_not_found timeout is a pre-dispatch timeout"
expect_missing "$FAKE_STATE_DIR/prompt.txt" "agent_not_found timeout sends no prompt"
expect_elapsed_between 1 4 "agent_not_found timeout is bounded"

# 12. A pane that never settles times out before dispatch.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"working"}}}'
run_wake w1:p1 'never settles' --timeout 1 --interval 1
expect_rc 3 "pre-dispatch timeout"
expect_contains "$STDERR" 'did not settle' "pre-dispatch timeout is labeled"
expect_missing "$FAKE_STATE_DIR/prompt.txt" "pre-dispatch timeout sends no prompt"
expect_elapsed_between 1 4 "pre-dispatch timeout is bounded near --timeout"

# 13. Malformed agent state is a parse error.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":"eight"}}}'
run_wake w1:p1 'bad state' --timeout 5 --interval 1
expect_rc 4 "invalid completion_seq"
expect_contains "$STDERR" 'invalid completion_seq' "invalid completion_seq is named"

# 14-16. Usage errors exit 4 before any Herdr call.
new_case
run_wake w1:p1
expect_rc 4 "missing prompt"
expect_contains "$STDERR" 'Usage:' "missing prompt prints usage"

new_case
run_wake w1:p1 'prompt' --bogus
expect_rc 4 "unknown option"
expect_contains "$STDERR" 'unknown option' "unknown option is named"

new_case
run_wake w1:p1 'prompt' --timeout 0
expect_rc 4 "zero timeout"
expect_contains "$STDERR" 'positive integer' "zero timeout is rejected"

# 17. Quotes and shell metacharacters survive as one literal argument.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
# shellcheck disable=SC2016 # the literals must reach the script unchanged
PROMPT_ARG='quotes "double" and '\''single'\'' and $HOME `cmd`'
run_wake w1:p1 "$PROMPT_ARG" --timeout 5 --interval 1
expect_rc 0 "quoted prompt"
expect_file "$FAKE_STATE_DIR/prompt.txt" "$PROMPT_ARG" "quoted prompt arrives verbatim"

# 18. A leading dash is a prompt, not an option.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
run_wake w1:p1 --timeout 5 --interval 1 -- '-leading dash "quoted"'
expect_rc 0 "leading dash prompt"
expect_file "$FAKE_STATE_DIR/prompt.txt" '-leading dash "quoted"' "leading dash prompt arrives verbatim"

# 19. A multi-line prompt is flattened to one line before dispatch.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
MULTILINE=$'first "quoted" line\nsecond\tline\r\nthird\ffourth'
FLAT='first "quoted" line second line third fourth'
run_wake w1:p1 "$MULTILINE" --timeout 5 --interval 1
expect_rc 0 "multi-line prompt"
expect_file "$FAKE_STATE_DIR/prompt.txt" "$FLAT" "multi-line prompt is dispatched as one line"

# 20. --lines reaches herdr pane read.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
run_wake w1:p1 'show lines' --timeout 5 --interval 1 --lines 7
expect_rc 0 "lines forwarding"
expect_file "$FAKE_STATE_DIR/pane-lines" '7' "lines value reaches pane read"

# 21. A failed pane read is reported, not swallowed as an empty pane.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
export FAKE_PANE_FAIL=1
run_wake w1:p1 'read failure' --timeout 5 --interval 1
expect_rc 0 "pane read failure keeps the wake exit"
expect_contains "$STDERR" 'ERROR: herdr pane read failed for pane w1:p1' \
  "pane read failure reports the script's own error"
expect_file "$STDOUT" '' "pane read failure leaves stdout empty"

# 22. A turn that ended early wakes through the working-to-settled fallback.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.seq $'{"result":{"agent":{"agent_status":"working"}}}\n{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
export FAKE_PANE_TEXT='stopped mid-thought'
run_wake w1:p1 'end early' --timeout 5 --interval 1
expect_rc 0 "working-to-settled fallback"
expect_contains "$STDERR" 'settled after working' "fallback reports the settled turn"
expect_contains "$STDOUT" 'stopped mid-thought' "fallback prints pane output"

# 23. The fallback does not fire when no working state was observed.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
run_wake w1:p1 'never started' --timeout 2 --interval 1
expect_rc 3 "fallback needs working"
expect_contains "$STDERR" 'TIMEOUT:' "idle-only stays a timeout"

# 24. A single idle read after working is not enough for the fallback.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.seq $'{"result":{"agent":{"agent_status":"working"}}}\n{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}\n{"result":{"agent":{"agent_status":"working"}}}'
run_wake w1:p1 'idle then working' --timeout 4 --interval 1
expect_rc 3 "working-idle-working does not wake"
expect_contains "$STDERR" 'TIMEOUT:' "working-idle-working stays a timeout"
expect_not_contains "$STDERR" 'WAKE:' "working-idle-working never reports a wake"

# 25. `unknown` after dispatch is not proof of completion.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"unknown","completion_seq":7}}}'
run_wake w1:p1 'cannot classify' --timeout 2 --interval 1
expect_rc 3 "unknown stays a timeout"
expect_contains "$STDERR" 'TIMEOUT:' "unknown is bounded by the timeout"
expect_not_contains "$STDERR" 'WAKE:' "unknown never reports a wake"

# 26. `unknown` before dispatch is not a settled baseline.
new_case
fixture before.seq $'{"result":{"agent":{"agent_status":"unknown"}}}\n{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
run_wake w1:p1 'unknown then idle' --timeout 5 --interval 1
expect_rc 0 "unknown is skipped before dispatch"
expect_contains "$STDERR" 'completion_seq 7 -> 8' "unknown does not become the baseline"

# 27. A hanging herdr agent get is bounded by the whole-command timeout.
new_case
export FAKE_GET_HANG=1
run_wake w1:p1 'hung get' --timeout 1 --interval 1
expect_rc 3 "hanging agent get hits the command bound"
expect_contains "$STDERR" 'command bound' "hanging get reports the command bound"
expect_elapsed_between 1 3 "hanging agent get is bounded"
expect_missing "$FAKE_STATE_DIR/prompt.txt" "hanging agent get sends no prompt"

# 28. A hanging herdr agent prompt is bounded by the whole-command timeout.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
export FAKE_PROMPT_HANG=1
run_wake w1:p1 'hung prompt' --timeout 3 --interval 1
expect_rc 3 "hanging agent prompt hits the command bound"
expect_contains "$STDERR" 'command bound' "hanging prompt reports the command bound"
expect_contains "$STDERR" 'while dispatching the prompt' "hanging prompt names the dispatch phase"
expect_elapsed_between 2 5 "hanging agent prompt is bounded"

# 29. A hanging pane read is bounded by the remaining command budget and the
# wake exit is kept. The case bound is tighter than the fake's sleep so an
# unbounded read fails instead of hanging the suite.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"idle","completion_seq":7}}}'
fixture after.json '{"result":{"agent":{"agent_status":"idle","completion_seq":8}}}'
export FAKE_PANE_HANG=1
run_wake_bounded 10 w1:p1 'hung pane read' --timeout 5 --interval 1
expect_rc 0 "hanging pane read keeps the wake exit"
expect_contains "$STDERR" 'herdr pane read' "hanging pane read is named"
expect_contains "$STDERR" 'exceeded the command bound' "hanging pane read reports the command bound"
expect_file "$STDOUT" '' "hanging pane read leaves stdout empty"
expect_elapsed_between 3 7 "hanging pane read is bounded by the remaining budget"

# 30. A hanging pane read on a blocked exit is bounded and keeps exit 2.
new_case
fixture before.json '{"result":{"agent":{"agent_status":"blocked","completion_seq":3}}}'
export FAKE_PANE_HANG=1
run_wake_bounded 10 w1:p1 'blocked with hung read' --timeout 5 --interval 1
expect_rc 2 "hanging pane read keeps the blocked exit"
expect_contains "$STDERR" 'BLOCKED:' "hanging pane read still reports blocked"
expect_contains "$STDERR" 'exceeded the command bound' "hanging blocked read reports the command bound"
expect_file "$STDOUT" '' "hanging blocked read leaves stdout empty"
expect_elapsed_between 3 7 "hanging blocked read is bounded by the remaining budget"

printf 'herdr-wake: %d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
