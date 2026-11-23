#!/usr/bin/env bash
# Clone or update a tap repo in /tmp/lthiagol-taps and print its path.
#
#   sync.sh homebrew-tap | homebrew-dev
#
# A clone that is dirty, not on main, or not a fast-forward of origin/main is
# left as it is: the script prints why and exits 1. Never resets.
set -euo pipefail

repo=${1-}
case ${repo} in
  homebrew-tap | homebrew-dev) ;;
  *) echo "usage: sync.sh homebrew-tap|homebrew-dev" >&2; exit 2 ;;
esac

base=/tmp/lthiagol-taps
dir=${base}/${repo}
mkdir -p "${base}"

if [[ ! -d "${dir}/.git" ]]
then
  git clone -q "git@github.com:lthiagol/${repo}.git" "${dir}"
else
  cd "${dir}"
  if [[ -n "$(git status --porcelain)" ]]
  then
    echo "sync.sh: ${dir} has uncommitted changes:" >&2
    git status --short >&2
    exit 1
  fi
  branch=$(git branch --show-current)
  [[ "${branch}" == main ]] || { echo "sync.sh: ${dir} is on '${branch}', not main" >&2; exit 1; }
  git fetch -q origin
  if ! git merge --ff-only -q origin/main 2>/dev/null
  then
    echo "sync.sh: ${dir} main has diverged from origin/main:" >&2
    git log --oneline origin/main..main >&2
    exit 1
  fi
fi

git -C "${dir}" config user.name "Luiz Thiago"
git -C "${dir}" config user.email lthiagol@gmail.com
echo "${dir}"
