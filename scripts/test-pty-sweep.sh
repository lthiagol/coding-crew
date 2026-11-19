#!/usr/bin/env bash
# Agent-runnable tests for scripts/pty-sweep.py.
#
# The tests put a fake `raul` behind --raul. It records the width, the height
# and the work folder of every call, prints a fixed number of rows, and can be
# told to overflow, fail, or print too few rows. No real terminal is touched.
#
# Every case runs under a wall-clock bound, so a missing script timeout fails
# the test instead of hanging it.
#
# Dependencies: bash, python3, coreutils (mktemp, timeout, tr), findutils,
# sed, awk.
set -uo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SWEEP="$HERE/pty-sweep.py"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/pty-sweep-test.XXXXXX")
CASE_TIMEOUT=${CASE_TIMEOUT:-60}
if ! [[ "$CASE_TIMEOUT" =~ ^[1-9][0-9]*$ ]]; then
  CASE_TIMEOUT=60
fi
trap 'rm -rf "$TMP"' EXIT

BIN="$TMP/bin"
mkdir -p "$BIN"
cat >"$BIN/raul" <<'FAKE'
#!/usr/bin/env bash
set -u
state=${FAKE_STATE_DIR:?FAKE_STATE_DIR not set}
width=""
height=""
work=""
while (($#)); do
  case "$1" in
    --render | --plain)
      shift
      ;;
    --width)
      width=${2:-}
      shift 2
      ;;
    --height)
      height=${2:-}
      shift 2
      ;;
    --*)
      printf 'fake raul: unknown option %s\n' "$1" >&2
      exit 2
      ;;
    *)
      work=$1
      shift
      ;;
  esac
done
printf '%s|%s|%s\n' "$width" "$height" "$work" >>"$state/calls"

if [[ -n ${FAKE_HANG_WIDTH:-} && $width == "$FAKE_HANG_WIDTH" ]]; then
  exec sleep 60
fi

if [[ -n ${FAKE_FAIL_WIDTH:-} && $width == "$FAKE_FAIL_WIDTH" ]]; then
  printf 'fake raul: simulated failure at %s\n' "$width" >&2
  exit 3
fi

rows=$height
if [[ -n ${FAKE_SHORT_WIDTH:-} && $width == "$FAKE_SHORT_WIDTH" ]]; then
  rows=$((height - 1))
fi

for ((row = 1; row <= rows; row++)); do
  columns=$width
  if [[ $row == 1 && -n ${FAKE_OVERFLOW_WIDTH:-} && $width == "$FAKE_OVERFLOW_WIDTH" ]]; then
    columns=$((width + 1))
  fi
  if [[ $row == 1 && -n ${FAKE_WIDE_WIDTH:-} && $width == "$FAKE_WIDE_WIDTH" ]]; then
    # Twenty wide characters are forty columns; the extra x overflows 40.
    for ((cell = 0; cell < 20; cell++)); do printf '界'; done
    printf 'x\n'
    continue
  fi
  if [[ $row == 1 && -n ${FAKE_COMBINING_WIDTH:-} && $width == "$FAKE_COMBINING_WIDTH" ]]; then
    # Each e plus its combining mark is one column.
    for ((cell = 0; cell < width; cell++)); do printf 'e\u0301'; done
    printf '\n'
    continue
  fi
  printf '%*s\n' "$columns" '' | tr ' ' 'x'
done
FAKE
chmod +x "$BIN/raul"

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
  mkdir -p "$FAKE_STATE_DIR/tmp"
  export FAKE_STATE_DIR
  unset FAKE_FAIL_WIDTH FAKE_SHORT_WIDTH FAKE_OVERFLOW_WIDTH FAKE_HANG_WIDTH
  unset FAKE_WIDE_WIDTH FAKE_COMBINING_WIDTH
  # The default output folder is a fresh one under TMPDIR; keep it inside the
  # case so the suite's own trap removes it.
  export TMPDIR="$FAKE_STATE_DIR/tmp"
  STDOUT="$FAKE_STATE_DIR/stdout"
  STDERR="$FAKE_STATE_DIR/stderr"
  RC=0
}

run_sweep() {
  timeout "$CASE_TIMEOUT" python3 "$SWEEP" "$@" >"$STDOUT" 2>"$STDERR"
  RC=$?
  if ((RC == 124)); then
    bad "case $CASE exceeded ${CASE_TIMEOUT}s wall-clock bound (script timeout missing or broken)"
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

expect_exists() {
  local path=$1 label=$2
  if [[ -e $path ]]; then
    ok
  else
    bad "$label: $path should exist"
  fi
}

expect_missing() {
  local path=$1 label=$2
  if [[ ! -e $path ]]; then
    ok
  else
    bad "$label: $path should not exist"
  fi
}

expect_line() {
  local file=$1 number=$2 want=$3 label=$4 got
  got=$(sed -n "${number}p" "$file")
  if [[ "$got" == "$want" ]]; then
    ok
  else
    bad "$label: line $number is '$got', want '$want'"
  fi
}

expect_count() {
  local want=$1 got=$2 label=$3
  if [[ "$got" == "$want" ]]; then
    ok
  else
    bad "$label: got $got, want $want"
  fi
}

# 1. The success path: every width 40-120, one file per width, the default
# height passed through, and a fresh temp folder reported and kept.
new_case
run_sweep --raul "$BIN/raul" --work mock-data/coding-crew/work
expect_rc 0 "success"
expect_contains "$STDOUT" 'sweep: 81 widths 40-120, height 40: every line fits' "success summary"
OUT=$(sed -n 's/^sweep output: //p' "$STDOUT")
expect_exists "$OUT" "default output folder"
if [[ "$OUT" == "$FAKE_STATE_DIR/tmp/raul-pty-sweep-"* ]]; then
  ok
else
  bad "default output folder is not a fresh temp folder: $OUT"
fi
expect_count 81 "$(find "$OUT" -maxdepth 1 -type f -name '*.txt' | wc -l)" "one file per width"
for width in 40 41 60 100 120; do
  expect_exists "$OUT/$width.txt" "width $width file"
done
expect_missing "$OUT/39.txt" "below-range file"
expect_missing "$OUT/121.txt" "above-range file"
expect_count 40 "$(wc -l <"$OUT/40.txt")" "default rows in width 40"
expect_count 0 "$(awk 'length($0) != 40 { count++ } END { print count + 0 }' "$OUT/40.txt")" "width 40 rows are 40 columns"
expect_count 81 "$(wc -l <"$FAKE_STATE_DIR/calls")" "one call per width"
expect_line "$FAKE_STATE_DIR/calls" 1 '40|40|mock-data/coding-crew/work' "first call"
expect_line "$FAKE_STATE_DIR/calls" 81 '120|40|mock-data/coding-crew/work' "last call"
# The folder is reported, not cleaned: it still holds the files above.
expect_exists "$OUT" "output folder is kept"

# 2. An explicit --out is used, created with its parents, and a --height is
# passed through.
new_case
run_sweep --raul="$BIN/raul" --height=12 --out="$FAKE_STATE_DIR/nested/out" --work=work
expect_rc 0 "explicit output and height"
expect_contains "$STDOUT" "sweep output: $FAKE_STATE_DIR/nested/out" "explicit output is reported"
expect_count 81 "$(find "$FAKE_STATE_DIR/nested/out" -maxdepth 1 -type f -name '*.txt' | wc -l)" "explicit output files"
expect_count 12 "$(wc -l <"$FAKE_STATE_DIR/nested/out/40.txt")" "height reaches the render"
expect_line "$FAKE_STATE_DIR/calls" 1 '40|12|work' "flags reach the render"
expect_count 0 "$(awk -F'|' '$2 != 12 || $3 != "work" { count++ } END { print count + 0 }' "$FAKE_STATE_DIR/calls")" "every call takes the height and work folder"

# 3. A line one column too wide fails the command and names the width, the
# line and the measured length. The failing file is still written.
new_case
export FAKE_OVERFLOW_WIDTH=57
run_sweep --raul "$BIN/raul" --work work
expect_rc 1 "overflow"
expect_contains "$STDERR" 'width 57' "overflow names the width"
expect_contains "$STDERR" 'line 1' "overflow names the line"
expect_contains "$STDERR" '58 columns' "overflow names the measured length"
expect_contains "$STDERR" '(limit 57)' "overflow names the limit"
expect_contains "$STDERR" '1 width failed' "overflow summary counts the failing width"
OUT=$(sed -n 's/^sweep output: //p' "$STDOUT")
expect_exists "$OUT/57.txt" "overflowing width file is written"
expect_count 58 "$(awk 'NR == 1 { print length($0) }' "$OUT/57.txt")" "overflowing row is one column too wide"
expect_exists "$OUT/40.txt" "widths before the overflow are written"
expect_exists "$OUT/120.txt" "widths after the overflow are written"
expect_not_contains "$STDERR" 'width 40' "no overflow is reported at 40"

# 4. A render that exits nonzero fails the command and names the width and
# the exit.
new_case
export FAKE_FAIL_WIDTH=77
run_sweep --raul "$BIN/raul" --work work
expect_rc 1 "raul failure"
expect_contains "$STDERR" 'width 77' "raul failure names the width"
expect_contains "$STDERR" 'exited 3' "raul failure names the exit"
expect_contains "$STDERR" 'simulated failure' "raul failure surfaces stderr"
OUT=$(sed -n 's/^sweep output: //p' "$STDOUT")
expect_exists "$OUT/77.txt" "failing width file is written"

# 5. An output that is shorter than the requested height fails: an overflow
# cannot hide in rows that were never drawn.
new_case
export FAKE_SHORT_WIDTH=63
run_sweep --raul "$BIN/raul" --height 12 --work work
expect_rc 1 "short render"
expect_contains "$STDERR" 'width 63' "short render names the width"
expect_contains "$STDERR" 'printed 11 rows' "short render counts the rows"
expect_contains "$STDERR" 'expected 12' "short render names the height"

# 6. The measurement is terminal columns, not characters: twenty wide
# characters overflow 40 where a character count would fit, and a combining
# mark costs no column.
new_case
export FAKE_WIDE_WIDTH=40
run_sweep --raul "$BIN/raul" --work work
expect_rc 1 "wide characters"
expect_contains "$STDERR" 'width 40: line 1 is 41 columns' "wide characters take two columns"

new_case
export FAKE_COMBINING_WIDTH=40
run_sweep --raul "$BIN/raul" --work work
expect_rc 0 "combining characters"
expect_contains "$STDOUT" 'every line fits' "a combining mark takes no column"

# 7. A render that hangs is bounded: the gate fails instead of stalling.
new_case
export FAKE_HANG_WIDTH=40
run_sweep --raul "$BIN/raul" --timeout 1 --work work
expect_rc 1 "hung render"
expect_contains "$STDERR" 'width 40' "hung render names the width"
expect_contains "$STDERR" 'did not finish within 1s' "hung render names the bound"
OUT=$(sed -n 's/^sweep output: //p' "$STDOUT")
expect_exists "$OUT/40.txt" "hung width file is written"
expect_exists "$OUT/120.txt" "widths after the hung one are written"

# 8. The work folder is optional: omitted, raul receives no positional.
new_case
run_sweep --raul "$BIN/raul"
expect_rc 0 "no work folder"
expect_count 0 "$(awk -F'|' '$3 != "" { count++ } END { print count + 0 }' "$FAKE_STATE_DIR/calls")" "no work folder means no positional"

# 9. A bad height or timeout is refused before any render.
new_case
run_sweep --raul "$BIN/raul" --height 0
expect_rc 2 "zero height"
expect_contains "$STDERR" 'positive number' "zero height is named"
expect_missing "$FAKE_STATE_DIR/calls" "zero height runs no render"

new_case
run_sweep --raul "$BIN/raul" --height 1001
expect_rc 2 "over-limit height"
expect_contains "$STDERR" 'positive number' "over-limit height is named"
expect_missing "$FAKE_STATE_DIR/calls" "over-limit height runs no render"

new_case
run_sweep --raul "$BIN/raul" --height x
expect_rc 2 "non-numeric height"
expect_contains "$STDERR" 'not a number' "non-numeric height is named"
expect_missing "$FAKE_STATE_DIR/calls" "non-numeric height runs no render"

new_case
run_sweep --raul "$BIN/raul" --timeout 0
expect_rc 2 "zero timeout"
expect_contains "$STDERR" 'positive number of seconds' "zero timeout is named"
expect_missing "$FAKE_STATE_DIR/calls" "zero timeout runs no render"

# 10. An unknown flag is refused, and --help reports the range and the flags.
new_case
run_sweep --raul "$BIN/raul" --bogus
expect_rc 2 "unknown flag"
expect_contains "$STDERR" 'unrecognized arguments' "unknown flag is named"

new_case
run_sweep --help
expect_rc 0 "help"
expect_contains "$STDOUT" '40 to 120' "help names the range"
expect_contains "$STDOUT" '--work' "help names the work flag"
expect_contains "$STDOUT" '--out' "help names the output flag"
expect_contains "$STDOUT" '--timeout' "help names the timeout flag"

printf 'pty-sweep: %d passed, %d failed\n' "$PASS" "$FAIL"
((FAIL == 0))
