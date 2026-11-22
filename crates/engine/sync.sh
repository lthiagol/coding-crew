#!/usr/bin/env bash
#
# sync.sh - compare the vendored leaf markdown engine with its pinned upstream.
#
# The upstream pin is fixed: tag 1.29.2, commit
# 29f6ad480d95a4240c63f63d390e9909afdad9e8. This script never follows a
# branch and never rewrites the vendored files on its own.
#
#   ./sync.sh check
#       Fetch the pin, compare every src/markdown module, print a unified diff
#       for each file that differs, list missing and extra files, verify both
#       SHA-256 columns in VENDORED.md against the bytes on disk, and check the
#       result against the deviation manifest. The manifest must have exactly
#       18 unique rows; every row must map the canonical upstream path
#       src/markdown/<name>.rs to the canonical vendored path
#       crates/engine/src/markdown/<name>.rs, with one shared module name, no
#       two rows may name the same vendored file, no row may name a file absent
#       from upstream, and every deviation number must match a numbered ledger
#       heading. Exit 0 only when the vendored bytes, the upstream bytes, the
#       manifest, and the ledger agree.
#
#   ./sync.sh update
#       Fetch the pin, print the same diffs, then stop. Nothing is written.
#
#   ./sync.sh update --yes
#       Fetch the pin, print every diff, then stop. --yes alone never writes.
#
#   ./sync.sh update --yes --force
#       Fetch the pin, print every diff, and only then copy upstream over the
#       vendored modules. Missing modules are copied too; extra local modules
#       are never deleted. The copied files lose the deviations the manifest
#       records for them, and the script names those before it copies.
#       --force refuses unless the check above passes, because a clean check is
#       what shows the vendored copy has no local changes the manifest does not
#       record. Add --discard-unrecorded to overwrite anyway. VENDORED.md is not
#       rewritten, so the checksum table and the numbered deviations must be
#       updated by hand.

set -euo pipefail

REPO_URL="https://github.com/rivolink/leaf.git"
TAG="1.29.2"
COMMIT="29f6ad480d95a4240c63f63d390e9909afdad9e8"
COMMIT_DATE="2026-10-25"
UPSTREAM_DIR="src/markdown"
LOCAL_DIR="src/markdown"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENDORED="$ROOT/VENDORED.md"

usage() {
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' \
        "${BASH_SOURCE[0]}"
}

die() {
    echo "sync.sh: $*" >&2
    exit 2
}

mode="check"
assume_yes=0
force=0
discard_unrecorded=0
for arg in "$@"; do
    case "$arg" in
        check) mode="check" ;;
        update) mode="update" ;;
        --yes) assume_yes=1 ;;
        --force) force=1 ;;
        --discard-unrecorded) discard_unrecorded=1 ;;
        -h | --help) usage; exit 0 ;;
        *) die "unknown argument '$arg'; try --help" ;;
    esac
done

[[ -f "$VENDORED" ]] || die "VENDORED.md is missing at $VENDORED"
command -v git >/dev/null || die "git is required"
command -v diff >/dev/null || die "diff is required"
command -v sha256sum >/dev/null || die "sha256sum is required"

# The manifest row is:
# | `src/markdown/x.rs` | `sha` | `crates/engine/src/markdown/x.rs` | `sha` | deviations |
manifest_field() {
    local vendor_path="$1" field="$2"
    awk -F'|' -v target="$vendor_path" -v field="$field" '
        function trim(s) { gsub(/^[ \t`]+|[ \t`]+$/, "", s); return s }
        /^\| `src\/markdown\// {
            if (trim($4) == target) {
                if (field == "sha") print trim($3)
                else if (field == "vendored_sha") print trim($5)
                else print trim($6)
                exit
            }
        }
    ' "$VENDORED"
}

manifest_rows() {
    awk -F'|' '
        function trim(s) { gsub(/^[ \t`]+|[ \t`]+$/, "", s); return s }
        /^\| `src\/markdown\// {
            print trim($2) "\t" trim($4) "\t" trim($6)
        }
    ' "$VENDORED"
}

ledger_numbers() {
    awk '
        /^### [0-9]+\./ {
            sub(/^### /, "")
            sub(/\..*$/, "")
            print
        }
    ' "$VENDORED"
}

ledger_title() {
    awk -v number="$1" '
        $1 == "###" && $2 == number "." {
            sub(/^### [0-9]+\. /, "")
            print
            exit
        }
    ' "$VENDORED"
}

declare -A ledger_seen=()
while IFS= read -r number; do
    [[ -n "$number" ]] && ledger_seen[$number]=1
done < <(ledger_numbers)
(( ${#ledger_seen[@]} > 0 )) || die "VENDORED.md has no numbered deviation headings"

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/leaf-sync.XXXXXX")"
trap 'rm -rf "$tmpdir"' EXIT

echo "Fetching $REPO_URL"
echo "  tag    $TAG"
echo "  commit $COMMIT ($COMMIT_DATE)"

git init -q "$tmpdir/repo"
git -C "$tmpdir/repo" remote add origin "$REPO_URL"
git -C "$tmpdir/repo" fetch -q --depth 1 origin "$COMMIT"
git -C "$tmpdir/repo" checkout -q FETCH_HEAD

actual_commit="$(git -C "$tmpdir/repo" rev-parse HEAD)"
[[ "$actual_commit" == "$COMMIT" ]] ||
    die "fetched $actual_commit, expected $COMMIT"

upstream="$tmpdir/repo/$UPSTREAM_DIR"
[[ -d "$upstream" ]] || die "upstream $UPSTREAM_DIR is missing"

declare -a manifest_problems=()
declare -A manifest_seen=()
manifest_count=0
while IFS=$'\t' read -r upstream_path vendor_path deviations; do
    manifest_count=$((manifest_count + 1))
    upstream_name="$(basename "$upstream_path")"
    vendor_name="$(basename "$vendor_path")"
    if [[ -z "$vendor_path" ]]; then
        manifest_problems+=("row $manifest_count has no vendored path")
    elif [[ -n "${manifest_seen[$vendor_path]:-}" ]]; then
        manifest_problems+=("duplicate manifest row for $vendor_path")
    else
        manifest_seen[$vendor_path]=1
    fi
    if [[ "$upstream_path" != "$UPSTREAM_DIR/$upstream_name" ]]; then
        manifest_problems+=(
            "row $manifest_count names noncanonical upstream path $upstream_path"
        )
    fi
    if [[ "$vendor_path" != "crates/engine/$LOCAL_DIR/$upstream_name" ]]; then
        manifest_problems+=(
            "row $manifest_count names noncanonical vendored path $vendor_path"
        )
    fi
    if [[ "$upstream_name" != "$vendor_name" ]]; then
        manifest_problems+=(
            "row $manifest_count maps $upstream_path to $vendor_path (basename mismatch)"
        )
    fi
    if [[ ! -f "$tmpdir/repo/$upstream_path" ]]; then
        manifest_problems+=(
            "row $manifest_count names $upstream_path, which has no upstream file"
        )
    fi
    if [[ "$deviations" != "identical" && -n "$deviations" ]]; then
        IFS=',' read -ra deviation_numbers <<<"$deviations"
        for raw_number in "${deviation_numbers[@]}"; do
            number="${raw_number//[[:space:]]/}"
            if [[ ! "$number" =~ ^[0-9]+$ ]]; then
                manifest_problems+=(
                    "row $manifest_count has invalid deviation number '$raw_number'"
                )
            elif [[ -z "${ledger_seen[$number]:-}" ]]; then
                manifest_problems+=(
                    "row $manifest_count references deviation $number, which has no numbered heading"
                )
            fi
        done
    fi
done < <(manifest_rows)
if (( manifest_count != 18 )); then
    manifest_problems+=("expected 18 manifest rows, found $manifest_count")
fi

declare -a changed=() identical=() missing=() extra=() unlisted=() stale=() vendored_stale=() contract=()
changed_log="$tmpdir/changed.log"
: >"$changed_log"

while IFS= read -r source; do
    name="$(basename "$source")"
    vendor_path="crates/engine/$LOCAL_DIR/$name"
    target="$ROOT/$LOCAL_DIR/$name"
    deviations="$(manifest_field "$vendor_path" deviations)"

    if [[ -z "$deviations" ]]; then
        unlisted+=("$vendor_path")
        continue
    fi

    recorded_upstream_sha="$(manifest_field "$vendor_path" sha)"
    actual_upstream_sha="$(sha256sum "$source" | cut -d' ' -f1)"
    if [[ "$recorded_upstream_sha" != "$actual_upstream_sha" ]]; then
        stale+=("$vendor_path")
    fi

    if [[ ! -f "$target" ]]; then
        missing+=("$vendor_path")
        continue
    fi

    recorded_vendored_sha="$(manifest_field "$vendor_path" vendored_sha)"
    actual_vendored_sha="$(sha256sum "$target" | cut -d' ' -f1)"
    if [[ "$recorded_vendored_sha" != "$actual_vendored_sha" ]]; then
        vendored_stale+=("$vendor_path")
    fi

    if cmp -s "$source" "$target"; then
        identical+=("$vendor_path")
        if [[ "$deviations" != "identical" ]]; then
            contract+=("$vendor_path is marked as a deviation but is byte-identical")
        fi
    else
        changed+=("$vendor_path")
        printf '%s\n' "$vendor_path" >>"$changed_log"
        if [[ "$deviations" == "identical" ]]; then
            contract+=("$vendor_path is marked identical but differs from upstream")
        fi
        echo
        echo "--- diff: $name"
        diff -u \
            --label "upstream/$TAG/$UPSTREAM_DIR/$name" \
            --label "vendored/crates/engine/$LOCAL_DIR/$name" \
            "$source" "$target" || true
    fi
done < <(find "$upstream" -maxdepth 1 -type f -name '*.rs' | sort)

while IFS=$'\t' read -r upstream_path vendor_path _; do
    [[ -f "$ROOT/${vendor_path#crates/engine/}" ]] || continue
    [[ -f "$tmpdir/repo/$upstream_path" ]] || extra+=("$vendor_path")
done < <(manifest_rows)

while IFS= read -r local; do
    name="$(basename "$local")"
    [[ -f "$tmpdir/repo/$UPSTREAM_DIR/$name" ]] ||
        extra+=("crates/engine/$LOCAL_DIR/$name")
done < <(find "$ROOT/$LOCAL_DIR" -maxdepth 1 -type f -name '*.rs' | sort)

echo
echo "Upstream: $REPO_URL $TAG $COMMIT"
echo "Modules:  $(( ${#changed[@]} + ${#identical[@]} + ${#missing[@]} )) compared"
echo "Changed:  ${#changed[@]}"
echo "Identical:${#identical[@]}"
echo "Missing:  ${#missing[@]}"

status=0
if (( ${#missing[@]} )); then
    printf '  missing: %s\n' "${missing[@]}"
    status=1
fi
if (( ${#extra[@]} )); then
    printf '  extra:   %s\n' "${extra[@]}"
    status=1
fi
if (( ${#unlisted[@]} )); then
    printf '  not in VENDORED.md: %s\n' "${unlisted[@]}"
    status=1
fi
if (( ${#stale[@]} )); then
    printf '  upstream checksum mismatch in VENDORED.md: %s\n' "${stale[@]}"
    status=1
fi
if (( ${#vendored_stale[@]} )); then
    printf '  vendored checksum mismatch in VENDORED.md: %s\n' "${vendored_stale[@]}"
    status=1
fi
if (( ${#contract[@]} )); then
    printf '  manifest conflict: %s\n' "${contract[@]}"
    status=1
fi
if (( ${#manifest_problems[@]} )); then
    printf '  manifest row: %s\n' "${manifest_problems[@]}"
    status=1
fi

if [[ "$mode" == "check" ]]; then
    if (( status == 0 )); then
        echo "check: the vendored bytes, manifest, and ledger agree"
    else
        echo "check: FAILED - fix the manifest or the vendored copy" >&2
    fi
    exit "$status"
fi

if (( ${#changed[@]} == 0 && ${#missing[@]} == 0 )); then
    echo "update: nothing to copy"
    exit 0
fi

# The copy set is every upstream module that differs from, or is missing in,
# the vendored copy. The deviations to erase are the ones the manifest records
# for those files, so the warning below names what the copy will remove.
copy_names=()
declare -A erased_numbers=()
for source in "$upstream"/*.rs; do
    name="$(basename "$source")"
    target="$ROOT/$LOCAL_DIR/$name"
    if [[ -f "$target" ]] && cmp -s "$source" "$target"; then
        continue
    fi
    copy_names+=("$name")
    deviations="$(manifest_field "crates/engine/$LOCAL_DIR/$name" deviations)"
    if [[ -n "$deviations" && "$deviations" != "identical" ]]; then
        IFS=',' read -ra deviation_numbers <<<"$deviations"
        for raw_number in "${deviation_numbers[@]}"; do
            erased_numbers["${raw_number//[[:space:]]/}"]=1
        done
    fi
done

describe_erased() {
    local number
    (( ${#erased_numbers[@]} )) || return 0
    while IFS= read -r number; do
        printf '  %s. %s\n' "$number" "$(ledger_title "$number")"
    done < <(printf '%s\n' "${!erased_numbers[@]}" | sort -n)
}

if (( ! assume_yes )); then
    echo
    echo "update: no files were written."
    echo "Review the diffs above. If they are wanted, rerun with --yes --force;"
    echo "then update the checksum table and deviation ledger in VENDORED.md."
    exit 3
fi

if (( ! force )); then
    echo
    echo "update: no files were written."
    echo "--yes alone does not overwrite the vendored copy."
    echo "Rerun with --yes --force to replace it. That removes these deviations"
    echo "from the files it copies: ${copy_names[*]}"
    describe_erased
    exit 3
fi

if (( status != 0 && ! discard_unrecorded )); then
    echo
    echo "update: no files were written."
    echo "The check above reports problems, so --force could overwrite local"
    echo "changes the manifest does not record. Fix them, or rerun with"
    echo "--yes --force --discard-unrecorded to overwrite anyway."
    exit 3
fi

echo
echo "update: --force is replacing ${copy_names[*]}"
if (( ${#erased_numbers[@]} )); then
    echo "update: this removes these deviations from the copied files:"
    describe_erased
    echo "They have to be re-applied, then the gate rerun."
fi

for name in "${copy_names[@]}"; do
    echo "update: copying $name"
    cp "$upstream/$name" "$ROOT/$LOCAL_DIR/$name"
done

echo
echo "update: copied upstream files into crates/engine/$LOCAL_DIR"
echo "VENDORED.md was not changed. Update its checksum table and deviations,"
echo "re-apply the deviations listed above, run cargo fmt,"
echo "make ci, and ./sync.sh check."
