#!/usr/bin/env bash
# Agent-runnable test for the `install` target in the Makefile.
#
# It dry-runs `make -n install` (nothing is built or installed) and checks that
# talita, hugo and raul are all installed, and that the PATH note names talita.
# ROOT overrides the checkout the Makefile is read from.
set -uo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT=${ROOT:-$HERE/..}
fails=0

plan=$(make -C "$ROOT" -n install 2>&1) || {
  echo "FAIL: make -n install exited non-zero:"
  echo "$plan"
  exit 1
}

expect() {
  if ! grep -qF -- "$1" <<<"$plan"; then
    echo "FAIL: $2"
    fails=$((fails + 1))
  fi
}

expect "install --path crates/talita" "install does not install talita"
expect "install --path crates/hugo" "install does not install hugo"
expect "install --path crates/raul" "install does not install raul"
if ! grep -qE '^echo "add .* to PATH: .*talita' <<<"$plan"; then
  echo "FAIL: the PATH note does not name talita"
  fails=$((fails + 1))
fi

if [ "$fails" -gt 0 ]; then
  exit 1
fi
echo "install: ok"
