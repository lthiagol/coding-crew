---
name: homebrew
description: Publish coding-crew to lthiagol's Homebrew taps — add or bump the `coding-crew` formula in lthiagol/homebrew-tap (from a release tag) or `coding-crew-dev` in lthiagol/homebrew-dev (from a main commit), working in a disposable clone under /tmp. Use after tagging a release, when main should reach the dev formula, or for plan unit 101.
argument-hint: "release <vX.Y.Z> | dev [<commit>] | add"
---

# homebrew — coding-crew's formulae

`$SKILL` is this folder: `.claude/skills/homebrew` in the coding-crew repo.

| Formula | Tap | Repo | Source |
|---|---|---|---|
| `coding-crew` | `lthiagol/tap` | `lthiagol/homebrew-tap` | a release tag `vX.Y.Z` |
| `coding-crew-dev` | `lthiagol/dev` | `lthiagol/homebrew-dev` | a commit on `main` |

One formula installs every binary: `raul` and `hugo`, and `talita` once its
crate ships a CLI. The binaries must be on PATH — agents run `hugo --skill`
(TODO.md, "hugo is found on PATH").

## Rules

- **Never run a brew write** (install, uninstall, reinstall, upgrade, update,
  tap, trust…): the machine runs a live Homebrew. Read-only brew is fine.
  Bottles are built by the user with `~/coding/homebrew-tap/scripts/bottle.sh`.
- Work only in the clone `sync.sh` gives you, under `/tmp/lthiagol-taps/`.
  Never edit `~/coding/homebrew-tap` or `~/coding/homebrew-dev`.
- Commit as `Luiz Thiago <lthiagol@gmail.com>` (`sync.sh` sets it). Stage by
  name. Never force-push, rebase or amend.
- A release needs its tag pushed to `lthiagol/coding-crew` first, and `make
  check` green on it. If an input is missing or unclear, ask (AskUserQuestion,
  recommended option first).

## 1. Get the clone

```bash
dir=$("$SKILL"/scripts/sync.sh homebrew-tap)   # or homebrew-dev
```

Exit 1 (dirty, not on main, diverged): show its output and stop.

## 2a. add — the first time

Write `Formula/coding-crew.rb` (tap) or `Formula/coding-crew-dev.rb` (dev),
following the taps' Rust formulae (`frond.rb`, `andre-dev.rb`):

```ruby
class CodingCrew < Formula
  desc "Task management for coding agents, with TUIs for the work and the flow"
  homepage "https://github.com/lthiagol/coding-crew"
  url "https://github.com/lthiagol/coding-crew/archive/refs/tags/vX.Y.Z.tar.gz"
  sha256 "<sha256>"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "rust" => :build

  def install
    %w[raul hugo].each do |crate|
      system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/#{crate}"
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/raul --version")
    assert_match version.to_s, shell_output("#{bin}/hugo --version")
  end
end
```

The dev formula is `class CodingCrewDev`, with `url
".../archive/<full-sha>.tar.gz"`, a `version "<X.Y.Z>-dev.<YYYYMMDD>"` line (the
workspace version in `Cargo.toml`), the comment "rolling formula tracking
`main`", and `conflicts_with "coding-crew", because: "both install raul and
hugo"`.

Add a README row, alphabetical: `| [coding-crew](https://github.com/lthiagol/coding-crew) | <desc> |`.
Commit: `coding-crew <version> (new formula)`.

## 2b. release `<vX.Y.Z>` — bump the stable formula

1. `git ls-remote --exit-code https://github.com/lthiagol/coding-crew refs/tags/<tag>`.
2. url `https://github.com/lthiagol/coding-crew/archive/refs/tags/<tag>.tar.gz`;
   `sha256` from `curl -sfL <url> | sha256sum` — empty or failed stops the bump.
3. Edit only `url` and `sha256`. If a `bottle do … end` block is there, delete
   it and the blank line after; tell the user to rebuild bottles.
4. Commit: `coding-crew <X.Y.Z>`.

## 2c. dev `[<commit>]` — bump the dev formula

1. Default commit: `git ls-remote https://github.com/lthiagol/coding-crew refs/heads/main`.
   Use the full sha.
2. url `.../archive/<sha>.tar.gz`; `sha256` as above; `version` to
   `<workspace version>-dev.<YYYYMMDD>`.
3. Commit: `coding-crew-dev <version>`.

## 3. Push and watch CI

```bash
git -C "$dir" push -q origin main
gh run list -R lthiagol/<repo> --limit 1 --json databaseId,headSha   # match HEAD
gh run watch <id> -R lthiagol/<repo> --exit-status --interval 20
```

On failure: `gh run view <id> -R lthiagol/<repo> --log-failed | tail -40`, and
stop. Fix forward only when the cause is clearly this change.

## 4. Report

The commit, the CI result, and the user's next step:
`brew update && brew upgrade lthiagol/<tap>/<formula>` (first time:
`brew tap lthiagol/<tap> && brew install lthiagol/<tap>/<formula>`); for a
stable release, the bottle rebuild with `bottle.sh build coding-crew`.
