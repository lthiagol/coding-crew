#!/usr/bin/env bash
# Dispatch a prompt to a Herdr agent and wake when its completion_seq advances.
#
# Herdr 0.9.5 can miss a fast turn in `herdr agent wait --until idle`: the
# agent finishes, completion_seq advances, and the wait still runs to its
# timeout. This command polls `herdr agent get` instead, so a fast completion
# is not missed.
#
# Usage: scripts/herdr-wake.sh <pane-id> <prompt> [options]
#
# The prompt is dispatched as one line: CR, LF, tab and form feed become
# spaces. Use `--` before a prompt that begins with '-'.
#
# Options:
#   --timeout SECONDS   hard bound for the whole command (default 1800)
#   --interval SECONDS  poll interval (default 1)
#   --lines N           final pane output lines (default 40)
#
# Exit codes:
#   0  completion_seq advanced, or a working turn settled without advancing it.
#   2  the agent is blocked.
#   3  the command bound expired.
#   The final pane read runs under the budget left in the command. If it fails
#   or no budget remains, the exit above is kept, the missing output is
#   reported on stderr, and stdout is empty.
#   4  usage error, or Herdr failed
#
# Dependencies: bash, python3, herdr, timeout and sleep (coreutils).
set -uo pipefail

TIMEOUT=1800
INTERVAL=1
LINES=40
PANE=""
PROMPT=""
POSITIONAL=()

usage() {
  cat <<'EOF'
Usage: scripts/herdr-wake.sh <pane-id> <prompt> [options]

The prompt is dispatched as one line: CR, LF, tab and form feed become
spaces. Use `--` before a prompt that begins with '-'.

Options:
  --timeout SECONDS   hard bound for the whole command (default 1800)
  --interval SECONDS  poll interval (default 1)
  --lines N           final pane output lines (default 40)

Exit codes:
  0  completion_seq advanced, or a working turn settled without advancing it
  2  the agent is blocked
  3  the command bound expired
  4  usage error, or Herdr failed

The final pane read runs under the budget left in the command. If it fails or
no budget remains, the exit above is kept, the missing output is reported on
stderr, and stdout is empty.
EOF
}

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 4
}

while (($#)); do
  case "$1" in
    --timeout)
      (($# >= 2)) || die "--timeout needs a value"
      TIMEOUT=$2
      shift 2
      ;;
    --interval)
      (($# >= 2)) || die "--interval needs a value"
      INTERVAL=$2
      shift 2
      ;;
    --lines)
      (($# >= 2)) || die "--lines needs a value"
      LINES=$2
      shift 2
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
      die "unknown option: $1"
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

((${#POSITIONAL[@]} == 2)) || {
  usage >&2
  exit 4
}
PANE=${POSITIONAL[0]}
PROMPT=${POSITIONAL[1]}

if ! [[ "$TIMEOUT" =~ ^[0-9]+$ ]] || ((TIMEOUT <= 0)); then
  die "--timeout must be a positive integer"
fi
if ! [[ "$INTERVAL" =~ ^[0-9]+$ ]] || ((INTERVAL <= 0)); then
  die "--interval must be a positive integer"
fi
if ! [[ "$LINES" =~ ^[0-9]+$ ]] || ((LINES <= 0)); then
  die "--lines must be a positive integer"
fi

command -v timeout >/dev/null 2>&1 || die "timeout (coreutils) is required"

# Print "STATUS<TAB>COMPLETION_SEQ". An absent or null completion_seq is empty.
parse_agent_state() {
  python3 -c '
import json
import sys

try:
    doc = json.load(sys.stdin)
except Exception as exc:
    print(f"invalid herdr JSON: {exc}", file=sys.stderr)
    raise SystemExit(4)

if not isinstance(doc, dict):
    print("invalid herdr response: not an object", file=sys.stderr)
    raise SystemExit(4)

error = doc.get("error")
if error is not None:
    code = error.get("code") if isinstance(error, dict) else None
    if code == "agent_not_found":
        # Not ready yet: a fresh pane has no recognised agent for a moment.
        raise SystemExit(5)
    if isinstance(error, dict):
        message = error.get("message") or error.get("code") or json.dumps(error)
    else:
        message = str(error)
    print(f"herdr error: {message}", file=sys.stderr)
    raise SystemExit(4)

result = doc.get("result")
if not isinstance(result, dict):
    print("invalid herdr response: no result object", file=sys.stderr)
    raise SystemExit(4)

agent = result.get("agent")
if not isinstance(agent, dict):
    print("invalid herdr response: no agent object", file=sys.stderr)
    raise SystemExit(4)

status = agent.get("agent_status") or ""
seq = agent.get("completion_seq")
if seq is not None and (isinstance(seq, bool) or not isinstance(seq, int)):
    print(f"invalid completion_seq: {seq!r}", file=sys.stderr)
    raise SystemExit(4)

print(status + "\t" + ("" if seq is None else str(seq)))
'
}

STATUS=""
SEQ=""
READY=0
START=$SECONDS

# Seconds left in the whole-command bound; fail when the bound is spent.
remaining_budget() {
  local left=$((TIMEOUT - (SECONDS - START)))
  ((left > 0)) || return 1
  printf '%s' "$left"
}

# Sleep only as long as the remaining bound allows.
poll_sleep() {
  local left
  left=$(remaining_budget) || return 1
  ((left > INTERVAL)) && left=$INTERVAL
  sleep "$left"
}

# READY=0 means herdr has not recognised an agent in the pane yet
# (agent_not_found); that is a readiness state, not a Herdr failure.
read_state() {
  local budget raw state rc parse_rc
  budget=$(remaining_budget) || return 3
  # SIGKILL keeps a hung client from exceeding the whole-command bound;
  # GNU timeout reports 137 for a SIGKILL timeout, not the usual 124.
  raw=$(timeout --signal=KILL "$budget" herdr agent get "$PANE" 2>&1)
  rc=$?
  if ((rc == 124 || rc == 137)); then
    return 3
  fi
  state=$(printf '%s' "$raw" | parse_agent_state)
  parse_rc=$?
  if ((parse_rc == 5)); then
    READY=0
    STATUS=""
    SEQ=""
    return 0
  fi
  if ((parse_rc != 0)); then
    return 4
  fi
  if ((rc != 0)); then
    printf '%s\n' "$raw" >&2
    return 4
  fi
  READY=1
  STATUS=${state%%$'\t'*}
  SEQ=${state#*$'\t'}
  [[ "$SEQ" == "$state" ]] && SEQ=""
  return 0
}

print_pane() {
  local budget rc
  if ! budget=$(remaining_budget); then
    printf 'ERROR: no command-bound budget left to read pane %s; the final pane output is missing\n' "$PANE" >&2
    return 1
  fi
  # recent-unwrapped keeps logical lines whole instead of preserving soft wraps.
  timeout --signal=KILL "$budget" herdr pane read "$PANE" --source recent-unwrapped --lines "$LINES"
  rc=$?
  if ((rc == 124 || rc == 137)); then
    printf 'ERROR: herdr pane read for pane %s exceeded the command bound; the final pane output is missing\n' "$PANE" >&2
    return 1
  fi
  if ((rc != 0)); then
    printf 'ERROR: herdr pane read failed for pane %s\n' "$PANE" >&2
    return 1
  fi
  return 0
}

blocked_exit() {
  printf 'BLOCKED: %s (pane %s, status %s)\n' "$1" "$PANE" "${STATUS:-unknown}" >&2
  print_pane
  exit 2
}

timeout_before_dispatch() {
  printf 'TIMEOUT: pane %s did not settle within the %ss command bound before dispatch (status %s, completion_seq %s)\n' \
    "$PANE" "$TIMEOUT" "${STATUS:-unknown}" "${SEQ:-none}" >&2
  print_pane
  exit 3
}

timeout_after_dispatch() {
  printf 'TIMEOUT: pane %s completion_seq did not advance within the %ss command bound (baseline %s, last %s, status %s)\n' \
    "$PANE" "$TIMEOUT" "${BASELINE_SEQ:-none}" "${SEQ:-none}" "${STATUS:-unknown}" >&2
  print_pane
  exit 3
}

timeout_dispatch() {
  printf 'TIMEOUT: pane %s exceeded the %ss command bound while dispatching the prompt (baseline %s, status %s)\n' \
    "$PANE" "$TIMEOUT" "${BASELINE_SEQ:-none}" "${STATUS:-unknown}" >&2
  print_pane
  exit 3
}

BASELINE_SEQ=""
SETTLED=0

# Flatten the prompt so it can never submit early: a newline inside the text
# would strand the rest in the input box. Tabs and form feeds are flattened
# too so they cannot move focus or clear the input.
PROMPT=${PROMPT//$'\r\n'/ }
PROMPT=${PROMPT//$'\r'/ }
PROMPT=${PROMPT//$'\n'/ }
PROMPT=${PROMPT//$'\t'/ }
PROMPT=${PROMPT//$'\f'/ }
PROMPT=${PROMPT//$'\v'/ }

# Capture a baseline from settled work. If the pane is mid-turn, let that turn
# finish first so its completion_seq advance cannot be mistaken for ours. Two
# consecutive idle|done reads close the window where the next turn has started
# but herdr still reports the previous state.
while :; do
  read_state
  rc=$?
  case "$rc" in
    0) ;;
    3) timeout_before_dispatch ;;
    *) exit 4 ;;
  esac
  if ((READY == 0)); then
    SETTLED=0
  else
    case "$STATUS" in
      blocked)
        blocked_exit "agent is blocked before dispatch"
        ;;
      idle | done)
        SETTLED=$((SETTLED + 1))
        if ((SETTLED >= 2)); then
          BASELINE_SEQ=$SEQ
          break
        fi
        ;;
      unknown)
        # Herdr cannot classify the agent; that does not prove completion.
        # Keep waiting, and let the command bound end the wait.
        SETTLED=0
        ;;
      *)
        SETTLED=0
        ;;
    esac
  fi
  poll_sleep || timeout_before_dispatch
done

prompt_budget=$(remaining_budget) || timeout_before_dispatch
prompt_out=$(timeout --signal=KILL "$prompt_budget" herdr agent prompt "$PANE" "$PROMPT" 2>&1)
prompt_rc=$?
if ((prompt_rc == 124 || prompt_rc == 137)); then
  timeout_dispatch
fi
if ((prompt_rc != 0)); then
  if [[ "$prompt_out" == *agent_blocked* ]]; then
    STATUS=blocked
    blocked_exit "prompt was rejected because the agent is blocked"
  fi
  printf 'ERROR: herdr agent prompt failed: %s\n' "$prompt_out" >&2
  exit 4
fi

SAW_WORKING=0
SETTLED_AFTER_WORKING=0
while :; do
  read_state
  rc=$?
  case "$rc" in
    0) ;;
    3) timeout_after_dispatch ;;
    *) exit 4 ;;
  esac
  if ((READY == 0)); then
    SETTLED_AFTER_WORKING=0
  else
    # Blocked wins over completion: the agent needs the orchestrator.
    [[ "$STATUS" == blocked ]] && blocked_exit "agent is blocked after dispatch"

    case "$STATUS" in
      working)
        SAW_WORKING=1
        SETTLED_AFTER_WORKING=0
        ;;
      idle | done)
        if ((SAW_WORKING == 1)); then
          SETTLED_AFTER_WORKING=$((SETTLED_AFTER_WORKING + 1))
        fi
        ;;
      unknown)
        # Herdr cannot classify the agent; that does not prove completion.
        # Keep waiting, and let the command bound end the wait.
        SETTLED_AFTER_WORKING=0
        ;;
      *)
        SETTLED_AFTER_WORKING=0
        ;;
    esac

    if [[ -n "$SEQ" ]] && { [[ -z "$BASELINE_SEQ" ]] || ((SEQ > BASELINE_SEQ)); }; then
      printf 'WAKE: pane %s completion_seq %s -> %s\n' \
        "$PANE" "${BASELINE_SEQ:-none}" "$SEQ" >&2
      print_pane
      exit 0
    fi

    # Fallback for a turn that ended early: it was observed working and has
    # settled again twice in a row without completion_seq advancing. Two
    # reads match the baseline rule and reject a transient idle mid-turn.
    if ((SAW_WORKING == 1 && SETTLED_AFTER_WORKING >= 2)); then
      printf 'WAKE: pane %s settled after working without completion_seq advancing (completion_seq %s, status %s)\n' \
        "$PANE" "${SEQ:-none}" "$STATUS" >&2
      print_pane
      exit 0
    fi
  fi

  poll_sleep || timeout_after_dispatch
done
