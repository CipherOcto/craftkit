#!/usr/bin/env bash
# Mutation-test the gates: does each one actually catch the thing it claims?
#
# A gate that cannot fail is worse than no gate — it teaches people to trust a
# check that has never checked anything. Every rule below is broken on purpose,
# the gate is run, and the tree is restored.
#
# # Safety, and why there are so many words about it
#
# Two ways this script can lie, both of which it has already done once:
#
# 1. **A mutation that does not apply.** Three of the mutations below silently
#    did nothing once because an anchor string had moved. The output still
#    looked plausible — a gate was reported MISSED that had never been handed a
#    broken tree. So every mutation goes through `swap`, which exits with an
#    error if the text it is looking for is not there. A mutation that changes
#    nothing is a bug in this script, not a result.
# 2. **A tree left changed.** An earlier version backed each file up as it went,
#    so a second mutation to the same file captured an already-broken copy and
#    the run ended with a duplicated TOML block in `crates/model/Cargo.toml`.
#    So a pristine snapshot is taken *before* any mutation, the whole mutable
#    surface is restored between cases, and the run ends by comparing
#    checksums and failing loudly if anything differs.
#
# 3. **A run that reports failures and still succeeds.** The closing summary
#    printed `N gate(s) MISSED their mutation` in red and then exited 0. In CI
#    that is a green build with the instrument shouting inside it. So the exit
#    status is part of the result, and `--self-test` asserts the negative: it
#    runs one synthetic case that must miss and fails if the script does not.
#
# 4. **One swap, one gate.** `expect_catch` restores the tree after every
#    case, so a mutation serves exactly one gate. Two gates under one swap
#    means the second one runs against an unmutated tree, passes, and is
#    reported as MISSED — which looks exactly like a gap in the code and is
#    not one. It happened, in §60's case, and was found because the run exits
#    non-zero when a gate is missed. Write the swap again for the second gate.
#
# 5. **Do not run the other gates while this one is running.** This script
#    mutates the tree and restores it between cases, so a `cargo xtask ci` run
#    at the same time reads a half-mutated tree. It happened: the `tests`
#    phase failed while the `lists` phase passed, which is the signature of a
#    tree that was being changed underneath it rather than a tree that was
#    wrong. For the same reason, do not edit a file the list names while this
#    script runs: the restore at the end puts the file back the way it was
#    when the run started, and the edit is lost without a word.
#
#    And do not edit **this script** while it runs. Bash reads a script
#    incrementally, so an edit in the middle of a run leaves the shell
#    reading at a byte offset that is no longer the start of a command: the
#    next `swap` looks for an anchor that is not there, and the run stops
#    with "THE MUTATION DID NOT APPLY, SO THE RUN IS VOID". The run says so
#    — it is honest about it — but the run is gone, and it is gone after the
#    twenty minutes it took to get there. It happened, in §102's own
#    measurement.
#
#   scripts/mutation-test-gates.sh
#   scripts/mutation-test-gates.sh --check       # is the file list usable?
#   scripts/mutation-test-gates.sh --self-test   # exits 1; exits 2 if the self-test is broken

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

readonly GREEN=$'\033[32m'
readonly RED=$'\033[31m'
readonly YELLOW=$'\033[33m'
readonly RESET=$'\033[0m'

# Every file a mutation may write to.
#
# The list is a file the script *reads*, not an array it carries. §187: bash
# reads a script as it goes, so a case cannot edit the script it is running --
# and a list inside the script could not be broken. A file that is read is a
# file that can be mutated, and a mutation of this list is a mutation of the
# contract between the script and the restore.
#
# The gate `cargo xtask anchors` reads the same file, so there is one list and
# both doors read it.
readonly FILES_FILE="scripts/mutation-files.txt"

# The list, read from that file.
#
# The format is one path per line; a line that starts with `#` is a comment.
# Every path must be a file in the tree -- a blank line is not a file, and the
# message says which of the two it is -- and the list must not be empty. The
# script refuses all three rather than snapshotting nothing and saying so at
# the end. The gate reads the same file and holds the same rules, so the two
# never disagree about what the list says.
FILES=()
FILE_COUNT=0
while IFS= read -r path || [ -n "$path" ]; do
    case "$path" in
        '#'*) continue ;;
    esac
    if [ ! -f "$path" ]; then
        if [ -z "$path" ]; then
            printf '%sTHE FILE LIST HAS A BLANK LINE.%s\n' "$RED" "$RESET" >&2
            printf '  %s is the list; a blank line in it is not a path.\n' "$FILES_FILE" >&2
        else
            printf '%sTHE FILE LIST NAMES A FILE THAT IS NOT IN THE TREE.%s\n' "$RED" "$RESET" >&2
            printf '  %s lists %s, and there is no such file.\n' "$FILES_FILE" "$path" >&2
        fi
        exit 1
    fi
    FILES+=("$path")
    FILE_COUNT=$((FILE_COUNT + 1))
done < "$FILES_FILE"
if [ "$FILE_COUNT" -eq 0 ]; then
    printf '%sTHE FILE LIST IS EMPTY.%s\n' "$RED" "$RESET" >&2
    printf '  %s lists no files, so no mutation could be put back.\n' "$FILES_FILE" >&2
    exit 1
fi

# `--check` says whether the list is usable, and nothing else.
#
# The list's rules are the gate's rules, and the gate is the authority. This
# is the script's own half of them: the fast refusal before the gate runs,
# and a way to break it on purpose -- a case cannot run the script that runs
# the cases, so this is what a case runs. §187.
if [[ "${1:-}" == "--check" ]]; then
    printf 'the file list is usable: %s file(s) in %s\n' "$FILE_COUNT" "$FILES_FILE"
    exit 0
fi

# Created rather than edited, so restoring means removing them.
readonly EPHEMERAL=(
    crates/unlisted
    crates/ui/assets-probe.png
    crates/ui/build-probe.py
    crates/io/src/format/pending/probe.rs
)

SNAPSHOT="$(mktemp -d)"
cleanup() {
    restore
    rm -rf "$SNAPSHOT"
}
trap cleanup EXIT

snapshot() {
    for f in "${FILES[@]}"; do
        mkdir -p "$SNAPSHOT/$(dirname "$f")"
        cp -a "$f" "$SNAPSHOT/$f"
    done
}

restore() {
    for f in "${FILES[@]}"; do
        cp -a "$SNAPSHOT/$f" "$f" 2>/dev/null
        # `cp -a` preserves the *snapshot's* timestamp, which predates the
        # binaries this script's own gates just built. Cargo compares mtimes, so
        # after a run it would keep the mutated `xtask` and the next `cargo
        # xtask` would run stale code — a gate reporting on a build that is no
        # longer what is on disk. Touching after the copy is the fix, and the
        # bug was found because a real failure appeared here that no source in
        # the tree could explain.
        touch "$f" 2>/dev/null
    done
    for e in "${EPHEMERAL[@]}"; do
        rm -rf "$e"
    done
}

# A checksum of the **tree**, so the script can prove it changed nothing.
#
# **The whole tree**, not only the files a mutation writes to. §203: the old
# fingerprint walked the FILES list, so a file a gate *created* was invisible
# to it -- the script could leave something behind and still say the tree was
# unchanged. The claim was wider than the thing it computed.
#
# `git ls-files -co --exclude-standard` is the tree the script is talking
# about: the tracked files and the untracked ones, without the ones the
# build owns (target/ and the rest of .gitignore). The build writes into
# those on every gate, and a checksum that included them would differ every
# run for a reason that is not the script's.
fingerprint() {
    git ls-files -co --exclude-standard | sort | while read -r f; do
        printf '%s ' "$f"
        cksum < "$f" 2>/dev/null || printf 'missing'
    done
}

# Replace `old` with `new` in `file`; fail loudly if it did not match.
swap() {
    guarded "$1"
    # The python below exits non-zero when the anchor is not there. That has
    # to stop the *script*, not just the mutation: a case that changed
    # nothing is handed to the gate as an unmutated tree, the gate passes,
    # and the case is reported as a MISSED result — which is a result about
    # the gate, when it is a bug in this file. The header has said so since
    # the guard was added; §46 is the run where it did not.
    if ! python3 - "$1" "$2" "$3" <<'PYSWAP'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path).read()
if old not in text:
    sys.exit(
        "MUTATION DID NOT APPLY\n"
        f"  file: {path}\n"
        f"  looking for: {old!r}\n"
        "  This is a bug in scripts/mutation-test-gates.sh: the anchor moved.\n"
        "  A mutation that changes nothing makes every result below meaningless."
    )
open(path, "w").write(text.replace(old, new, 1))
PYSWAP
    then
        printf '\n%sTHE MUTATION DID NOT APPLY, SO THE RUN IS VOID.%s\n' "$RED" "$RESET" >&2
        printf '  Fix the anchor, or the case. The results below are about nothing.\n' >&2
        exit 1
    fi
}

# Append one line to a file.
append() {
    guarded "$1"
    printf '%s\n' "$2" >> "$1"
}

# A mutation may only touch a file the snapshot knows about.
#
# This guard exists because the whitelist had already let one through. `FILES`
# is a list, `restore` and `fingerprint` walk it, and a mutation against a file
# that was not on it was restored by nothing and checked by nothing: the tree
# kept the broken source, and the run still printed "Tree unchanged" and exited
# 0. What noticed was the next ordinary `cargo test` — not the script that made
# the mess, which is the worst way for it to be noticed.
#
# So `FILES` is the contract, not a convenience: a mutation outside it is a
# mutation this script cannot put back, and it refuses rather than guessing.
guarded() {
    local target="$1" known
    for known in "${FILES[@]}"; do
        [ "$known" = "$target" ] && return 0
    done
    printf '%sMUTATION TARGET IS NOT SNAPSHOTTED%s\n  %s\n\n%s\n%s\n' \
        "$RED" "$RESET" "$target" \
        "  It is not in the list of files a mutation may write to, so snapshot() never" \
        "  copied it, restore() cannot put it back and fingerprint() never looks at it." \
        "  Add it to $FILES_FILE first." >&2
    exit 1
}

# A copy of the script with one thing changed, for the gate to check.
#
# The script cannot be changed while it runs: bash reads a script as it goes,
# so an edit in the middle of a run leaves the shell reading at a byte offset
# that is not the start of a command (§102). A copy is a different file, and
# the copy goes in the snapshot directory, which the run deletes.
#
# The change is applied the way `swap` applies one, and it fails the same way:
# a copy that changes nothing is a copy that tests nothing. §187.
broken_copy() {
    local out="$1" from="$2" to="$3"
    if ! python3 - "$out" "$from" "$to" <<'PYCOPY'
import sys
out, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
text = open("scripts/mutation-test-gates.sh").read()
if old not in text:
    sys.exit(
        "BROKEN COPY DID NOT APPLY\n"
        f"  looking for: {old!r}\n"
        "  This is a bug in scripts/mutation-test-gates.sh: the anchor moved.\n"
        "  A copy that changes nothing tests nothing."
    )
open(out, "w").write(text.replace(old, new, 1))
PYCOPY
    then
        printf '\n%sTHE BROKEN COPY DID NOT APPLY, SO THE RUN IS VOID.%s\n' "$RED" "$RESET" >&2
        printf '  Fix the text, or the case. The results below are about nothing.\n' >&2
        exit 1
    fi
}

# The anchors, checked before anything is mutated. §184.
#
# The `swap` helper refuses an anchor that is not there, and it refuses it when
# the case is reached -- up to twenty-five minutes into a run, with the tree
# already changed twenty times, and the report says what the anchor was and
# never where it had been. So every anchor is checked first, all of them at
# once, and the report says where each one is.
#
# This is a gate of its own and not a phase of `lists`: a mutation removes the
# anchor it names -- that is what a mutation is -- so an anchor gate inside
# `lists` would report every case as caught, whatever the case was for.
check_anchors() {
    local out rc
    out="$(cargo xtask anchors 2>&1)"; rc=$?
    if [[ $rc -ne 0 ]]; then
        printf '\n%sTHE SCRIPT HAS AN ANCHOR THAT IS NOT WHERE IT SAYS.%s\n' "$RED" "$RESET" >&2
        printf '%s\n' "$out" >&2
        printf '  Every anchor is checked before the run starts, so this is found in a\n' >&2
        printf '  second rather than twenty-five minutes in. Fix the anchor, or the case.\n' >&2
        exit 1
    fi
}

declare -a CAUGHT=()
declare -a MISSED=()
declare -a BROKE=()
declare -a SKIPPED=()
CASES=0

# Does the output say the tree did not compile?
#
# §102: a gate that fails because the **build** broke has not tested anything.
# The gate rejects the tree, the script calls the case caught, and the rule
# the case claims to protect is untested — the test never ran, because there
# was nothing to run it against. A broken build is not a catch; it is a
# mutation that says nothing.
#
# The patterns are the ones cargo prints for a compilation failure, and not
# the ones it prints for a failing test: a test failure says "error: test
# failed, to rerun pass ...", which is a catch.
did_not_compile() {
    printf '%s' "$1" | grep -qE 'error\[E[0-9]+\]|error: could not compile|error: aborting due to'
}

# Expect a gate to REJECT the mutated tree.
#
# The third argument says whether a **compilation failure** is the catch. It is
# "no" by default, because a tree that does not compile has no test to run
# and so has not tested the rule. The one gate whose *subject* is a
# compilation says "yes": for it, a build that breaks is the thing it exists
# to notice. §102.
#
# The fourth argument names a **tool** the case needs. Where the tool is not
# there, the case is reported as **skipped** -- never as caught, and never as
# missed. A mutation that cannot run has tested nothing, and reporting it as
# caught would be the one lie this script exists to stop. §173.
#
# A skipped case is **not** counted in `CASES`. The closing ratio counts the
# cases that ran, so "384/384 caught" means every case that ran was caught,
# and the skipped ones are named underneath it. Counting a case that never ran
# in the denominator would dress it up as a tested one.
#
# The label is the first argument, and it is **always a literal**: a label the
# shell can rewrite is a label a reader cannot trust. §190.
expect_catch() {
    local label="$1" gate="$2" compile_is_catch="${3:-no}" tool="${4:-}"
    if [[ -n "$tool" ]] && ! command -v "$tool" >/dev/null 2>&1; then
        SKIPPED+=("$label -- $tool is not installed")
        printf '  %sskipped%s %s\n' "$YELLOW" "$RESET" "$label"
        printf '          %s is not installed; the rule was not tested\n' "$tool"
        return
    fi
    local out rc
    CASES=$((CASES + 1))
    out="$(eval "$gate" 2>&1)"; rc=$?
    restore
    if [[ $rc -ne 0 ]]; then
        if [[ "$compile_is_catch" == "yes" ]] || ! did_not_compile "$out"; then
            printf '  %scaught%s  %s\n' "$GREEN" "$RESET" "$label"
            CAUGHT+=("$label")
        else
            printf '  %sBROKE%s  %s\n' "$YELLOW" "$RESET" "$label"
            printf '          the tree did not compile, so no test ran: the rule is untested\n'
            BROKE+=("$label")
        fi
    else
        printf '  %sMISSED%s  %s\n' "$RED" "$RESET" "$label"
        printf '          %s\n' "$(printf '%s' "$out" | tail -1 | cut -c1-88)"
        MISSED+=("$label")
    fi
}

# Expect a gate NOT to reject the tree, for a rule it cannot enforce.
expect_miss() {
    local label="$1" gate="$2" why="$3" out rc
    CASES=$((CASES + 1))
    out="$(eval "$gate" 2>&1)"; rc=$?
    restore
    if [[ $rc -eq 0 ]]; then
        printf '  %sknown%s   %s\n' "$YELLOW" "$RESET" "$label"
        printf '          not catchable: %s\n' "$why"
    else
        printf '  %schanged%s %s — now caught; update the boundary in xtask/src/wasm.rs\n' \
            "$GREEN" "$RESET" "$label"
    fi
}

# Report what happened and exit with the truth.
#
# The exit status is part of the result, not a formality: this script is a gate,
# and a gate that prints its failures and then returns success is the one thing
# it exists to prevent. `finish` is a function so `--self-test` below can drive
# the same path rather than a lookalike of it.
finish() {
    local status=0
    echo
    if [[ $CASES -eq 0 ]]; then
        # Every case was skipped, so nothing was tested. A run that tested
        # nothing must not print a ratio: "0/0 caught" is a green light for a
        # dark room. §173.
        printf '%sno case ran.%s Every case was skipped; nothing was tested.\n' "$YELLOW" "$RESET"
        status=1
    elif [[ ${#MISSED[@]} -eq 0 && ${#BROKE[@]} -eq 0 ]]; then
        printf '%s%d/%d caught.%s The `known` line is a limit of wasm32 itself, not a gate that passed.\n' \
            "$GREEN" "${#CAUGHT[@]}" "$CASES" "$RESET"
    else
        if [[ ${#MISSED[@]} -gt 0 ]]; then
            printf '%s%d of %d gate(s) MISSED their mutation:%s\n' "$RED" "${#MISSED[@]}" "$CASES" "$RESET"
            for f in "${MISSED[@]}"; do printf '   • %s\n' "$f"; done
            printf '\n%sA gate that cannot fail is worse than no gate.%s Fix the gate, or the check.\n' \
                "$RED" "$RESET"
        fi
        if [[ ${#BROKE[@]} -gt 0 ]]; then
            printf '%s%d of %d mutation(s) broke the build, so no gate ran:%s\n' \
                "$RED" "${#BROKE[@]}" "$CASES" "$RESET"
            for f in "${BROKE[@]}"; do printf '   • %s\n' "$f"; done
            printf '\n%sA mutation that does not compile tests nothing.%s Fix the mutation, so it compiles.\n' \
                "$RED" "$RESET"
        fi
        status=1
    fi

    # A skipped case is reported as skipped. Never as caught, never as passed.
    if [[ ${#SKIPPED[@]} -gt 0 ]]; then
        printf '%s%d case(s) skipped, not passed:%s\n' "$YELLOW" "${#SKIPPED[@]}" "$RESET"
        for f in "${SKIPPED[@]}"; do printf '   • %s\n' "$f"; done
    fi

    if [[ "$(fingerprint)" != "$BEFORE" ]]; then
        printf '\n%sThe script left the tree changed.%s That is a bug in this script.\n' "$RED" "$RESET"
        status=1
    else
        printf '\nTree unchanged.\n\n'
    fi
    exit $status
}

# The self-test does not run the anchors check: that mode runs without any
# cargo at all, and it is about the exit status, not about the anchors.
if [[ "${1:-}" != "--self-test" ]]; then
    check_anchors
fi

snapshot
readonly BEFORE="$(fingerprint)"

# ------------------------------------------------------------- self-test ----
# Prove the script fails when a gate stops catching, without running any cargo.
#
# This is the negative of the whole exercise, so it is the one thing worth
# asserting cheaply: a synthetic case that is guaranteed to miss, driven through
# the real `expect_catch` and the real `finish`. If someone makes the exit
# status zero again, CI runs this and goes red.
if [[ "${1:-}" == "--self-test" ]]; then
    echo
    echo "-- self-test: a missed mutation must make this script fail"
    expect_catch "synthetic: a gate that never rejects" "true"
    if [[ ${#MISSED[@]} -eq 0 ]]; then
        printf '%sself-test FAILED: the missed mutation was not recorded at all%s\n' "$RED" "$RESET"
        exit 2
    fi
    # The other half, and the one that bit in §46: a mutation whose anchor is
    # not in the file must stop the run. Reported as a missed *gate*, it is a
    # result about the gate when it is a bug in this file. The subshell keeps
    # the snapshot alive for the parent, which still has a tree to restore.
    echo
    echo "-- self-test: a mutation that does not apply must stop the run"
    if ( swap docs/architecture.md 'a line that is not in the file at all' 'x' ) >/dev/null 2>&1; then
        printf '%sself-test FAILED: a mutation that did not apply did not stop the run%s\n' "$RED" "$RESET"
        exit 2
    fi
    # The third half, and it is §102: a gate that fails because the **build**
    # broke has not tested anything. The tree did not compile, so no test
    # ran, so the rule the case claims to protect is untested — and the script
    # used to call that a catch. A synthetic gate that prints what cargo
    # prints for a compilation failure is enough to prove the classification.
    echo
    echo "-- self-test: a gate that fails on a broken build is not a catch"
    expect_catch "synthetic: a gate that fails because the tree did not compile" \
        "bash -c 'echo \"error[E0308]: mismatched types\"; echo \"error: could not compile\"; exit 1'"
    if [[ ${#BROKE[@]} -eq 0 ]]; then
        printf '%sself-test FAILED: a broken build was counted as a caught mutation%s\n' "$RED" "$RESET"
        exit 2
    fi
    # The fourth half: the snapshot has to cover the files a **build**
    # writes, not only the files a mutation writes. Cargo rewrites
    # `Cargo.lock` when a manifest changes, and a file the snapshot does not
    # hold is a file the restore cannot put back and the fingerprint never
    # looks at -- so the tree is not what the script says it is, while the
    # script says it is. §163.
    echo
    echo "-- self-test: the snapshot must cover the files a build writes"
    for written in Cargo.lock; do
        if [[ ! " ${FILES[*]} " == *" ${written} "* ]]; then
            printf '%sself-test FAILED: %s is not in the snapshot%s\n' "$RED" "$written" "$RESET"
            printf '  A file a build writes is a file the restore has to put back.\n' >&2
            exit 2
        fi
    done
    # The fifth half: the claim has to see a file the run **created**. §203:
    # the fingerprint used to walk the list of files a mutation writes to, so
    # a file a gate created was invisible to it -- the script could leave
    # something behind and say the tree was unchanged. The claim was wider
    # than the thing it computed.
    echo
    echo "-- self-test: the claim must see a file the run created"
    probe="docs/fingerprint-probe.txt"
    printf 'left behind\n' > "$probe"
    with_probe="$(fingerprint)"
    rm -f "$probe"
    if [[ "$with_probe" == "$(fingerprint)" ]]; then
        printf '%sself-test FAILED: a file the run created is invisible to the claim%s\n' "$RED" "$RESET"
        exit 2
    fi
    finish
fi

echo
echo "── Does each gate catch the thing it claims?"
echo

# ---------------------------------------------------------------- layers ----
append crates/core/Cargo.toml 'craftkit-engine = { workspace = true }'
expect_catch "layers: L0 core gains an L4 dependency" "cargo xtask layers"

append crates/model/Cargo.toml 'egui = { workspace = true }'
expect_catch "layers: L1 model names egui" "cargo xtask layers"

# `AGENTS.md` says the shell depends on egui and nothing else - no windowing,
# no renderer - and the layer gate only refused a UI toolkit *below* L6. The
# shell is L6, so a window in the shell was not a violation of anything, and
# the rule had nothing behind it.
swap crates/ui/Cargo.toml \
    'egui = { workspace = true }' \
    'egui = { workspace = true }
eframe = { workspace = true }'
expect_catch "layers: the shell takes a window" "cargo xtask layers"

mkdir -p crates/unlisted/src
printf '[package]\nname = "craftkit-unlisted"\nversion.workspace = true\nedition.workspace = true\npublish = false\n' \
    > crates/unlisted/Cargo.toml
: > crates/unlisted/src/lib.rs
expect_catch "layers: a workspace crate with no table row" "cargo xtask layers"

# ---------------------------------------------------------------- assets ----
printf '\211PNG\r\n\032\nnot really a png' > crates/ui/assets-probe.png
expect_catch "assets: a PNG in the tree with no ATTRIBUTION row" "cargo xtask assets"

append crates/ui/src/lib.rs 'const _PROBE: &[u8] = include_bytes!("assets-probe.png");'
expect_catch "assets: include_bytes! of an image" "cargo xtask assets"

append ATTRIBUTION.md '| `ghost.png` | an icon that never existed | original |'
expect_catch "assets: an ATTRIBUTION row with no file" "cargo xtask assets"

# `AGENTS.md` says "Rust only", and nothing checked it: `is_asset` knows about
# binary and media formats, so a Python helper inside a crate, a stray `.js` in
# the web app and a `deploy.sh` all left `cargo xtask ci` green. The rule was
# also already untrue as written — the tree holds two shell scripts and the CI
# workflow beyond the two JavaScript files it named.
printf 'print("not rust")\n' > crates/ui/build-probe.py
expect_catch "assets: a Python file appears inside a crate" "cargo xtask assets"

# ----------------------------------------------------------------- lists ----
# Removing a line, so the replacement is empty rather than a no-op.
swap crates/kernel/src/lib.rs '#![forbid(unsafe_code)]' '// removed on purpose'

# Dropping one lint: still a deny attribute, still plausible, and `todo!()`
# would now compile in this crate.
swap crates/io/src/lib.rs '    clippy::todo,
' ''

expect_catch "lists: a crate root drops #![forbid(unsafe_code)]" "cargo xtask lists"

swap crates/io/src/lib.rs '    clippy::todo,
' ''
expect_catch "lists: a crate root drops one lint from its deny attribute" "cargo xtask lists"

swap docs/architecture.md '| L4 | `craftkit-engine` |' '| L7 | `craftkit-engine` |'
expect_catch "lists: the documented layer table moves a crate" "cargo xtask lists"

swap docs/architecture.md '| L5 | `craftkit-mcp` | core, model, engine |' '| L5 | `craftkit-mcp` | core, engine |'
expect_catch "lists: a dependency column drifts from cargo metadata" "cargo xtask lists"

swap Cargo.toml '    "apps/*",
    "crates/*",' '    "crates/*",
    "apps/*",'
expect_catch "lists: the workspace member list is unsorted" "cargo xtask lists"

# Prose points at code by line number, and line numbers rot the moment anything
# above them moves. The tree carried ninety such citations and nothing looked at
# any of them; five pointed at blank lines or braces and two more pointed at the
# import block of the file whose logic they claimed to quote. Some of that drift
# was caused by this audit editing the very files it cited.
append docs/architecture.md 'See `crates/core/src/lib.rs:9999` for the root.'
expect_catch "lists: prose cites a line that is not in the file" "cargo xtask lists"

# --------------------------------------------------------- the gates are real ----
# §169. Three claims: a gate the tree names exists, every command is documented,
# and every gate runs. A sibling repo's `AGENTS.md` named `cargo xtask
# cleanroom` and neither the command nor the step was there — a promise with
# nothing behind it.
#
# The first: the check that reads a command's name out of the prose. If it
# looks for the wrong phrase it finds nothing, and a document can promise a
# gate that does not exist for ever.
swap xtask/src/lists.rs \
    '    const PHRASE: &str = "cargo xtask ";' \
    '    const PHRASE: &str = "cargo xtaskx ";'
expect_catch "gates: a document names a gate that does not exist, and nothing notices" \
    "cargo test -p xtask --profile ci a_gate_the_prose_names_but_that_does_not_exist_is_caught"

# The second: a command that is in no table is a command nobody finds. `bundle`
# ran, and was in neither the usage text nor the crate's own table, for as long
# as the tree has had it.
swap xtask/src/lists.rs \
    '    for command in commands {
        let row = format!("//! | [`{name}`]", name = command.name);' \
    '    for command in commands.iter().filter(|command| command.name.is_empty()) {
        let row = format!("//! | [`{name}`]", name = command.name);'
expect_catch "gates: a command that is in no table is not noticed" \
    "cargo test -p xtask --profile ci a_command_the_table_does_not_list_is_caught"

# The third: whether a command is a gate is *derived* from the list of
# exceptions, so that the default is "it is a gate" and silencing one takes a
# line and a reason. If the derivation is lost, the exceptions list is a list of
# words, and `bundle` — which writes files — is a gate that `ci` runs.
swap xtask/src/lib.rs \
    'pub fn is_gate(name: &str) -> bool {
    !NOT_GATES.iter().any(|(not, _)| *not == name)
}' \
    'pub fn is_gate(name: &str) -> bool {
    let _ = name;
    true
}'
expect_catch 'gates: a command that `ci` does not run is treated as one it does' \
    "cargo xtask lists"

# The fourth: a document's table of gates, read as empty. Every phase of `ci`
# then looks undocumented, and the tables that say what protects what are
# worthless. The tree keeps more than one — the README's and the checking
# skill's — and this is the check that reads both.
swap xtask/src/lists.rs \
    '        let rows = gate_rows(&text);' \
    '        let rows: Vec<String> = Vec::new();'
expect_catch "gates: a document's table of gates is read as empty" \
    "cargo test -p xtask --profile ci a_gate_the_readme_lists_but_that_does_not_exist_is_caught"

# The fourth, second half: the checking skill numbers its sections, so its
# table of gates is under "## 2. The gates". Stop recognising the number and
# the skill's table is not read at all — and the gate says nothing, because a
# document it does not read cannot disagree with it.
swap xtask/src/lists.rs \
    '        Some((number, rest)) if number.chars().all(|c| c.is_ascii_digit()) => rest,' \
    '        Some((number, rest)) if number.chars().all(|c| c.is_ascii_digit()) => text,'
expect_catch "gates: the checking skill's table of gates stops being read" \
    "cargo test -p xtask --profile ci a_numbered_gates_section_is_the_gates_section"

# The fifth: the usage text's line for `ci` is a list beside the list it
# copies. Compared against nothing, it is never wrong, and the usage text can
# leave out a phase without anybody noticing.
swap xtask/src/lists.rs \
    '        let named = list_names(ci.what);
        if named != phases {' \
    '        let named = list_names(ci.what);
        if named.len() > 100 {'
expect_catch "gates: the usage text's list of the phases is compared against nothing" \
    "cargo test -p xtask --profile ci the_usage_text_must_name_the_phases_and_only_them"

# The sixth: and the same for every list of the phases in the prose. The
# README's own line is one of them, and since section 189 the rule reads
# every copy the same way -- so the case moves with the rule.
swap xtask/src/lists.rs \
    '            let named: Vec<&str> = named.iter().map(String::as_str).collect();
            if named != phases {' \
    '            let named: Vec<&str> = named.iter().map(String::as_str).collect();
            if named.len() > 100 {'
expect_catch "gates: a list of the phases in the prose is compared against nothing" \
    "cargo test -p xtask --profile ci the_readme_must_name_the_phases_and_only_them"

# The seventh: the usage text is written from the command table, so it cannot
# forget a command. Write it from part of the table and it forgets three —
# and `bundle`, which writes files into the tree, is one of them.
swap xtask/src/lib.rs \
    '    for command in COMMANDS {
        out.push_str(&format!("  {:<10} {}\n", command.name, command.what));
    }' \
    '    for command in COMMANDS.iter().take(6) {
        out.push_str(&format!("  {:<10} {}\n", command.name, command.what));
    }'
expect_catch "gates: the usage text is written from part of the command table" \
    "cargo test -p xtask --profile ci every_command_is_in_the_usage_text"

# The eighth: an exception with no reason is a gate that was silenced. The
# list is closed and every entry is checked, but the reason is what a reviewer
# reads, and an empty one says nothing.
swap xtask/src/lib.rs \
    '    ("bundle", "it writes files into the tree"),' \
    '    ("bundle", ""),'
expect_catch "gates: an exception that says no reason is accepted" "cargo xtask lists"

# ------------------------------------------------------ the tree has one walk ----
# §170. The tree's walk had four homes: one in `xtask/src/tree.rs` and three in
# `xtask/src/lists.rs`. The copies had already drifted — `collect_manifests`
# did not ignore `dist` while the other two did — so a gate that walks the
# tree and a gate that walks it differently do not see the same tree.
#
# The gate says a recursive directory walk has one home. These two plant a
# second one and require the gate to notice: the first in the gate's own
# file, the second in another, because a check that only looks in one place
# is a check that looks almost nowhere.
swap xtask/src/lists.rs \
    '#[cfg(test)]
mod tests {
    use super::*;' \
    '/// A second walk of the tree, planted to prove the gate notices.
fn a_second_walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            a_second_walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;'
expect_catch "gate: the tree grows a second walk of its files" "cargo xtask lists"

swap xtask/src/assets.rs \
    '#[cfg(test)]
mod tests {
    use super::*;' \
    '/// A second walk of the tree, planted to prove the gate notices.
fn another_walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            another_walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;'
expect_catch "gate: a second walk of the tree, in another file" "cargo xtask lists"

# --------------------------------------------------------------------- io ----
# A format change is one file in `format/pending/`, and the migration it holds
# only runs if it is also listed in the `FILES` table. An unlisted file was
# invisible to all four pending tests, because the test that looked like it
# checked this asserted only that the table was empty. The design then loads
# without its rewrite — not an error, just wrong values.
mkdir -p crates/io/src/format/pending
printf '//! Probe: a format change nobody registered.\n\npub fn step(doc: &mut craftkit_model::Document) {\n    let _ = doc;\n}\n' \
    > crates/io/src/format/pending/probe.rs
expect_catch "io: a format change on disk that the table never applies" \
    "cargo test -p craftkit-io --profile ci"

# ------------------------------------------------------------------ model ----
# The one class of thing this audit had not found: not a check that cannot fail,
# but a *computation nothing checked the answer to*. Replacing `easeInOut` and
# both bezier presets with a linear ramp — gutting the bisection solver, the
# most numerically involved code in the timeline — failed exactly one test in
# the workspace, and that one only covered `Hold`.
swap crates/model/src/keyframe.rs \
    '        Easing::EaseInOut => smoothstep(linear),
        Easing::Bezier { out_x, out_y } => bezier_progress(linear, out_x as f64, out_y as f64),' \
    '        Easing::EaseInOut => linear,
        Easing::Bezier { .. } => linear,'
expect_catch "model: every easing silently becomes linear" "cargo test -p craftkit-model --profile ci"

# ------------------------------------------------------------------- eval ----
# Two of the evaluator's load-bearing rules were anchored, but only inside a test
# named for the cache: "a source sets the level, it does not multiply it" and
# "the chain starts at unity". Both were caught, and both were one careless edit
# to an unrelated test away from being caught at all. Now each has its own name.
swap crates/eval/src/evaluate.rs '                    level_out = amount;' \
    '                    level_out = apply_gain(level_in, amount).unwrap_or(level_in);'
expect_catch "eval: a source multiplies instead of setting the level" \
    "cargo test -p craftkit-eval --profile ci"

swap crates/eval/src/evaluate.rs 'pub const START_LEVEL: f64 = 1.0;' \
    'pub const START_LEVEL: f64 = 0.5;'
expect_catch "eval: the chain stops starting at unity" "cargo test -p craftkit-eval --profile ci"

# ---------------------------------------------------------------- control ----
# The control channel is a token-guarded JSON endpoint that runs engine
# commands. `allow_remote` is the only thing between it and every interface on
# the machine, and its default was untested: the loopback test passed a hardcoded
# `false` instead of the real default, so flipping it left all 41 tests green.
swap crates/control/src/server.rs \
    'Self { port, token, allow_remote: false, ui: None, after: None, token_dir: None }' \
    'Self { port, token, allow_remote: true, ui: None, after: None, token_dir: None }'
expect_catch "control: the default stops being loopback-only" \
    "cargo test -p craftkit-control --profile ci"

# The realistic way the boundary goes is not a flipped default but somebody
# setting the flag where the server is built.
append apps/craftkit/src/main.rs \
    '// let options = ServerOptions { allow_remote: true, ..Default::default() };'
expect_catch "control: a binary sets the flag that opens the channel to the network" \
    "cargo test -p craftkit-control --profile ci"

# A command that runs on the socket thread leaves the window painting the frame
# it already had, because the session is behind a mutex the frame loop never
# sees. The hook is the only thing that tells it. Dropping `Method::Execute`
# from it is exactly the bug: `ui.*` still repaints, every command silently
# does not, and no test in the workspace fails unless one asks whether the hook
# fires.
swap crates/control/src/server.rs \
    'matches!(method, Method::Execute | Method::Ui)' \
    'matches!(method, Method::Ui)'
expect_catch "control: a command stops telling the window the document changed" \
    "cargo test -p craftkit-control --profile ci"

# A document naming a command the registry has never heard of. The nearest real
# command to the one `docs/recipes/add-a-command.md` used to list is `node.mute`.
#
# This case does two jobs, and the second is the reason it is written in a
# subdirectory. `prose_files` did not recurse, so the walk behind every
# `file.rs:12` check opened `docs/*.md` — three files — and never one of the
# fifteen under `recipes/`, `design/` and `patterns/`, 49 citations in all. It
# said nothing about that, because a walk that looks almost nowhere looks like a
# walk that looked everywhere.
#
# Planting the defect where only a *recursive* walk can see it is what locks the
# recursion down. If someone flattens the walk again, this case stops being
# caught, the run reports it MISSED and exits non-zero — which is the instrument
# doing the one thing it is for.
#
# There is deliberately no second case for "remove the recursion". Removing it
# genuinely changes what the gate can see, so the gate passing is then correct
# and no mutation can honestly assert otherwise; a case written to pretend
# otherwise would be a gate asserting that it is worse than it is.
append docs/recipes/add-an-icon.md \
    'The shape matches `node.frobnicate`.'
expect_catch "lists: a document names a command that does not exist" "cargo xtask lists"

# `craftkit-cli mcp` is the one subcommand no test executed, because it blocks on
# stdio — which is what a pipe is for. The server promised in its own doc
# comment to end "or `shutdown` arrives", acknowledged `shutdown` with a
# success, and then served the next request anyway: a method that reported
# success without doing the thing it names.
swap crates/mcp/src/server.rs \
    '    let stop = request.method == "shutdown";' \
    '    let stop = false;'
expect_catch "mcp: shutdown says ok and then keeps serving" \
    "cargo test -p craftkit-cli --profile ci --test mcp_stdio"

# The other half: the stop must not travel inside the reply, or a client that
# sends `shutdown` as a notification would be ignored.
swap crates/mcp/src/server.rs \
    '    Answer { reply: if is_notification { None } else { Some(response) }, stop }' \
    '    Answer { reply: if is_notification { None } else { Some(response) }, stop: false }'
expect_catch "mcp: a shutdown sent as a notification is silently ignored" \
    "cargo test -p craftkit-mcp --profile ci shutdown"

# ------------------------------------------------------------------- mcp ----
# §171. The advertisement and the dispatch are one list now, and these two
# break the link in each direction.
#
# The first: the dispatch looks the tool up in the wrong case, so every tool
# the server advertises is a tool it refuses to call. This is the family's
# defect -- in one repo of the family, `tools/list` advertised five tools and
# `tools/call` answered to seventeen, so twelve of them were invisible to a
# client that discovers tools the way MCP clients do.
swap crates/mcp/src/server.rs \
    '    if let Some(built_in) = tools::built_in(name) {' \
    '    if let Some(built_in) = tools::built_in(&name.to_uppercase()) {'
expect_catch "mcp: the dispatch looks the tool up in the wrong case" \
    "cargo test -p craftkit-mcp --profile ci every_advertised_tool_can_be_called"

# The second: the other direction. A tool the server can call but does not
# advertise is a tool no client finds, and it is the half the first mutation
# cannot see.
swap crates/mcp/src/tools.rs \
    '    BUILT_INS
        .iter()
        .map(|built_in| Tool {' \
    '    BUILT_INS
        .iter()
        .filter(|built_in| built_in.name != "craftkit_crashes")
        .map(|built_in| Tool {'
expect_catch "mcp: a built-in is callable but not advertised" \
    "cargo test -p craftkit-mcp --profile ci every_built_in_is_advertised"

# `craftkit-cli crashes` reads the crash folder a person is told to open first
# when the app dies, and on a machine where nothing has crashed that folder is
# empty — so the command had never run against a real report.
# `CRAFTKIT_CRASH_DIR` is what makes the whole pipeline testable without
# touching anybody's home, and nothing else depends on it.
swap crates/engine/src/crash.rs \
    '    if let Some(dir) = std::env::var_os(CRASH_DIR_ENV) {' \
    '    if let Some(dir) = std::env::var_os("CRAFTKIT_NOT_THE_CRASH_DIR") {'
expect_catch "crash: the crash folder can no longer be relocated, so the pipeline is untestable" \
    "cargo test -p craftkit-cli --profile ci --test crash_pipeline"

# The crash report is a **reproduction**, and the four ways it used to stop
# being one were all silent. §47.
#
# The verdict first: a replay that ends somewhere else has to say so. With the
# comparison gone, the reply is "reproduced" for every report, including the
# ones that reproduced nothing.
swap crates/engine/src/crash.rs \
    '        reproduced: differences.is_empty(),' \
    '        reproduced: true,'
expect_catch "crash: a replay that does not reproduce the design is called a reproduction" \
    "cargo test -p craftkit-engine --profile ci replay"

# The base. The recorded commands are relative to the document they ran
# against, so a replay that starts from an empty session rebuilds a document
# that never existed — and said nothing about it.
swap crates/engine/src/crash.rs \
    '    session.load_document(report.base);' \
    '    let _ = report.base;'
expect_catch "crash: the replay starts from an empty session instead of the report's base" \
    "cargo test -p craftkit-engine --profile ci replay"

# The parameters. The trace used to be a line of text cut off after a couple of
# hundred characters, and the replay ran the command with whatever survived the
# parse — an empty parameter object, usually.
swap crates/engine/src/session.rs \
    '        let step = TraceStep { command: id.to_string(), params: params.clone() };' \
    '        let step = TraceStep { command: id.to_string(), params: serde_json::Value::Object(Default::default()) };'
expect_catch "crash: the trace keeps the parameters" \
    "cargo test -p craftkit-engine --profile ci keeps_the_parameters_exactly"

# The trace has to start again when a document is loaded, or it holds commands
# that were relative to a document that is gone.
swap crates/engine/src/session.rs \
    '        self.history.clear();
        self.log.clear();
        self.reset_trace();' \
    '        self.history.clear();
        self.log.clear();'
expect_catch "crash: opening a document does not start the command trace again" \
    "cargo test -p craftkit-engine --profile ci starts_the_trace_again"

# The undo stack belongs to the document it was made from. Without this, the
# first Ctrl+Z after opening a file replaced the file with the previous
# document's history.
swap crates/engine/src/session.rs \
    '        self.history.clear();
        self.log.clear();
        self.reset_trace();' \
    '        self.log.clear();
        self.reset_trace();'
expect_catch "crash: opening a document keeps the undo stack of the previous one" \
    "cargo test -p craftkit-engine --profile ci clears_the_undo_stack"

# And the same verdict at the surface a person reads: the CLI.
swap apps/craftkit-cli/src/cli.rs \
    '    if outcome.reproduced {' \
    '    if true {'
expect_catch "crash: the CLI calls a divergence a reproduction" \
    "cargo test -p craftkit-cli --profile ci --test crash_pipeline"

# A bounded trace cannot hold every command, so the report says how many it
# dropped. Without the count, a partial replay reads as a whole one.
swap crates/engine/src/crash.rs \
    '        dropped: session.trace_dropped(),' \
    '        dropped: 0,'
expect_catch "crash: a trace that dropped commands does not say so" \
    "cargo test -p craftkit-engine --profile ci dropped"

# A crash report is *input*: the recipe asks for `report.json` from whoever
# had the crash, so the file arrives from another machine. §48.
swap crates/engine/src/crash.rs \
    '    let body = read_bounded(path, MAX_REPORT_BYTES)?;' \
    '    let body = std::fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;'
expect_catch "crash: a report file of any size is read" \
    "cargo test -p craftkit-engine --profile ci larger_than_the_limit"

# And the same for the steps, which the replay *runs*. The cost is quadratic,
# so the bound has to be checked before the first step, not after.
swap crates/engine/src/crash.rs \
    '    if report.replay.len() > crate::MAX_COMMAND_STEPS {' \
    '    if false {'
expect_catch "crash: a replay runs as many steps as the report holds" \
    "cargo test -p craftkit-engine --profile ci too_many_steps"

# And the base has to be a document the model would accept. The design reader
# refuses a document with two nodes sharing an id; this one did not.
swap crates/engine/src/crash.rs \
    '    report.base = checked_document(report.base, "the report'"'"'s base design")?;' \
    '    let _ = &report.base;'
expect_catch "crash: a report with a corrupt base is replayed anyway" \
    "cargo test -p craftkit-engine --profile ci corrupt_base"

# A report is **input**: it arrives from another machine. Its design used to
# be read as a JSON tree, and a tree costs thirty-two times the text it was
# parsed from — a 66 000 101-byte report cost 1 260 292 KiB of resident
# memory. Read as a document, the same report costs 67 796 KiB, and a report
# whose design is not a document is refused by the format instead of being
# materialised first. §72.
#
# The mutation is the old shape exactly: the file is parsed into a tree, and
# the report is read out of the tree. It compiles — the report's own types
# did not change — so only a *measurement* can catch it.
swap crates/engine/src/crash.rs \
    '    let body = read_bounded(path, MAX_REPORT_BYTES)?;
    let mut report: CrashReport = serde_json::from_str(&body)
        .map_err(|e| format!("{} is not a readable report: {e}", path.display()))?;' \
    '    let body = read_bounded(path, MAX_REPORT_BYTES)?;
    let tree: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("{} is not a readable report: {e}", path.display()))?;
    let mut report: CrashReport = serde_json::from_value(tree)
        .map_err(|e| format!("{} is not a readable report: {e}", path.display()))?;'
expect_catch "crash: the report is read into a JSON tree before it is read as a report" \
    "cargo test -p craftkit-engine --profile ci reading_a_report_costs"

# Every limit in this tree was checked in the *parser*, after the bytes were
# already in memory. §49.
#
# The shared reader is the one that applies the limit while reading. Take the
# limit away and the file is read whole before it is refused — the memory the
# cap was meant to prevent is spent anyway.
swap crates/core/src/readers.rs \
    '    reader.take(max as u64 + 1).read_to_string(&mut out).map_err(io_error)?;' \
    '    reader.take(u64::MAX).read_to_string(&mut out).map_err(io_error)?;'
expect_catch "io: a bounded read is not bounded" \
    "cargo test -p craftkit-core --profile ci a_bounded_read_never_asks_for_more_than_the_limit"

# The line reader keeps at most `max` bytes and drops the rest; without the
# drop, one long line is one long allocation. The first version of this case
# mutated a branch that the *error* also depended on, and the test passed
# anyway -- the error was the same either way, and the difference was in the
# memory. The mutation now moves the property the test can see.
swap crates/core/src/readers.rs \
    '    let take = room.min(chunk.len());' \
    '    let take = chunk.len();'
expect_catch "io: a long line is kept instead of dropped" \
    "cargo test -p craftkit-core --profile ci the_bytes_kept_never_exceed_the_limit"

# The engine's document and the file's document are the same thing. §50.
#
# The engine's backstop: a command that leaves a document the file reader
# would refuse has to be refused and rolled back.
swap crates/engine/src/session.rs \
    '                    if let Err(e) = self.doc.validate() {
                        self.restore(snapshot, before_dirty);
                        let err = EngineError::Internal {
                            id: id.to_string(),
                            detail: format!(
                                "the command left the document in a state the file format refuses: {e}"
                            ),
                        };
                        self.log.record(id, false, &err.to_string());
                        return Err(err);
                    }
' \
    ''
expect_catch "engine: a command may leave a document the file reader refuses" \
    "cargo test -p craftkit-engine --profile ci a_command_that_leaves_an_invalid_document_is_refused_and_rolled_back"

# The model's group-id rule. It was stricter than its own reason: two groups
# with the same id under one parent are ambiguous, and the same id at two
# depths is not -- and a command can create that.
swap crates/model/src/property.rs \
    '            if let PropNode::Group(g) = node
                && siblings.iter().any(|id| id.eq_ignore_ascii_case(&g.id))
            {
                return Err(CoreError::LimitReached { limit: "duplicate group id" });
            }' \
    '            if let PropNode::Group(g) = node
                && false
            {
                return Err(CoreError::LimitReached { limit: "duplicate group id" });
            }'
expect_catch "model: two groups with one id are allowed among siblings" \
    "cargo test -p craftkit-model --profile ci two_siblings_with_the_same_id_are_refused"

# The same rule, the other way round: the *old* rule, restored exactly. The
# group ids are threaded through the recursion, so the check is tree-wide
# again -- and the same id at two depths is refused. This is the mutation
# that says the "a group id may repeat at a different depth" test is really
# asking the question it claims to ask.
swap crates/model/src/property.rs \
    '        self.check(0, &mut prop_ids, &mut 0)?;' \
    '        let mut group_ids: Vec<String> = Vec::new();
        self.check(0, &mut prop_ids, &mut group_ids, &mut 0)?;'
swap crates/model/src/property.rs \
    '    fn check(&self, depth: usize, prop_ids: &mut Vec<u64>, total: &mut usize) -> Result<(), CoreError> {' \
    '    fn check(
        &self,
        depth: usize,
        prop_ids: &mut Vec<u64>,
        group_ids: &mut Vec<String>,
        total: &mut usize,
    ) -> Result<(), CoreError> {'
swap crates/model/src/property.rs \
    '        let mut siblings: Vec<&str> = Vec::new();
        for node in &self.nodes {
            if let PropNode::Group(g) = node
                && siblings.iter().any(|id| id.eq_ignore_ascii_case(&g.id))
            {
                return Err(CoreError::LimitReached { limit: "duplicate group id" });
            }
            if let PropNode::Group(g) = node {
                siblings.push(&g.id);
            }
        }' \
    '        for node in &self.nodes {
            if let PropNode::Group(g) = node
                && group_ids.iter().any(|id| id.eq_ignore_ascii_case(&g.id))
            {
                return Err(CoreError::LimitReached { limit: "duplicate group id" });
            }
            if let PropNode::Group(g) = node {
                group_ids.push(g.id.clone());
            }
        }'
swap crates/model/src/property.rs \
    '                    g.check(depth + 1, prop_ids, total)?;' \
    '                    g.check(depth + 1, prop_ids, group_ids, total)?;'
expect_catch "model: the same group id at two depths is refused again" \
    "cargo test -p craftkit-model --profile ci a_group_id_may_repeat_at_a_different_depth"

# The group a path names is the group that is created: the id is the name.
#
# The first version of this mutation changed the group's *label* and left the
# id alone -- and the test passed. The resolver matches on the id
# (`index_of_seg` compares with `eq_ignore_ascii_case`), so a group called
# `trigger#2` under the id `trigger` still answered to `trigger/lead`, and
# the mutation measured nothing. The id is what the path names, so the id is
# what the mutation moves.
#
# The group is made by the model now -- the engine used to split the path
# string itself -- so the mutation moved with it. §85.
swap crates/model/src/property.rs \
    '                    self.push_group(PropGroup::new(name.clone(), name.clone()));' \
    '                    self.push_group(PropGroup::new(format!("{}#2", name), name.clone()));'
expect_catch "engine: a group is created under a name the path does not name" \
    "cargo test -p craftkit-cli --profile ci a_group_name_at_two_depths_survives_the_file"

# The writer must not write what the reader refuses.
swap crates/io/src/store.rs \
    '    doc.validate().map_err(|e| SaveError::Format(FormatError::InvalidDocument(e.to_string())))?;' \
    ''
expect_catch "io: the writer writes a document the reader would refuse" \
    "cargo test -p craftkit-io --profile ci the_writer_refuses_a_document_the_reader_would_refuse"

# The reader has a limit and the writer did not, so the writer could produce a
# file the reader refuses. §51.
#
# The limit is the whole of the fix: take it away and the writer goes back to
# producing text of any size.
swap crates/io/src/format.rs \
    '        self.to_text_bounded(MAX_FILE_BYTES)' \
    '        self.to_text_bounded(usize::MAX)'
expect_catch "io: the writer writes a file too large to read back" \
    "cargo test -p craftkit-io --profile ci a_design_whose_file_would_be_too_big_is_not_written"

# The limit has to stop the text *while* it is written, not after: §49's
# lesson, in the writer. A writer that keeps the chunk it refused has spent
# the memory the limit was meant to prevent.
swap crates/io/src/format.rs \
    '        if self.seen > self.max {' \
    '        if false {'
expect_catch "io: the writer keeps the bytes it refused" \
    "cargo test -p craftkit-io --profile ci the_writer_keeps_no_more_than_the_limit"

# Every limit in the model was a count, and the product of the counts was
# never written down. §52.
#
# The document's byte limit: a document over it is refused. The mutation
# compares against a number nothing can reach, so the check never fires --
# and the variable stays used, so the test fails for the right reason rather
# than because the tree would not compile.
swap crates/model/src/document.rs \
    '        if bytes > limits::MAX_DOCUMENT_BYTES {' \
    '        if bytes > usize::MAX {'
expect_catch "model: a document over the byte limit is accepted" \
    "cargo test -p craftkit-model --profile ci a_document_over_the_byte_limit_is_refused"

# The estimate the budget is measured in. An estimate that does not count the
# content is not an estimate, and a budget measured in one is not a budget.
#
# The first version of this mutation zeroed `Value::estimated_bytes`, which
# under-counted the values and was MISSED: the estimate was still large
# enough for the budget to bind, so nothing noticed. The count that matters
# is the keyframes' — that is where a document's memory goes.
#
# That last sentence was itself the defect §73 found: the keyframes are where
# an *honest* document's memory goes, and the property's own value was not
# counted at all, so a document full of large values was estimated at
# kilobytes. The comment is corrected below, and the value has its own
# mutation.
swap crates/model/src/property.rs \
    '                .map(|k| std::mem::size_of::<Key>() + k.value.estimated_bytes())' \
    '                .map(|_k| 0)'
expect_catch "model: the size estimate does not count the content" \
    "cargo test -p craftkit-engine --profile ci the_history_keeps_its_memory_within_a_budget"

# The property's own value, which the estimate did not count. §73.
#
# Measured: a document whose properties held 6 556 800 bytes of text was
# estimated at 30 258 bytes, so the undo budget (64 MiB) never bound and the
# stack grew to 128 steps and 830 MB of memory. With the value counted the
# same document is estimated at 6 587 058 bytes and the stack holds 9.
swap crates/model/src/property.rs \
    '            + self.label.capacity()
            + self.value.estimated_bytes()' \
    '            + self.label.capacity()'
expect_catch "model: the size estimate does not count the property's value" \
    "cargo test -p craftkit-model --profile ci the_size_estimate_counts_the_property_value"

# The value's invariants are kept by its constructors, and a file is
# deserialized straight into the model: no constructor runs, so nothing
# checked the number. A file could carry a number a hundred million times
# past the limit, and the evaluator's comment says every number the tree
# stores is in range. §73.
swap crates/model/src/property.rs \
    '                    p.value.check(&p.label)?;' \
    ''
expect_catch "model: a value that breaks an invariant is accepted" \
    "cargo test -p craftkit-io --profile ci a_file_whose_value_is_out_of_range_is_refused"

# The format document's example file. §79: "That is a complete, loadable
# file. `craftkit-cli eval` on it reports one node, 12 frames at 24 fps, and
# no diagnostics." — three claims about a hand-written file, in the one
# document that describes the format, and nothing checked them.
swap docs/design/file-format.md \
    '"meta": { "name": "Hand written", "fps": 24,' \
    '"meta": { "name": "Hand written", "fps": 30,'
expect_catch "docs: the format's example file is not what the document says" \
    "cargo test -p craftkit-cli --profile ci the_formats_example_file_is_what_the_document_says"

# A boundary that catches a panic, and the pattern document that declares
# it. §78: the document said "two boundaries, with distinct jobs, and no
# more", and the tree had three — the control channel's `ui.*` handler was
# argued for in the code and missing from the document — and §76 added a
# fourth. A boundary has to be written down before it is written.
swap xtask/src/lists.rs \
    '        "crates/engine/src/session.rs",
        "crates/control/src/server.rs",
        "apps/craftkit/src/main.rs",' \
    '        "crates/engine/src/session.rs",
        "apps/craftkit/src/main.rs",'
expect_catch "docs: a boundary that catches a panic is not declared" \
    "cargo xtask lists"

# A number of commands in prose, that is not the registry's number. §71's
# lesson: a number written by hand about a thing that grows is stale by
# construction. The registry grows — every command added makes the number in
# the prose wrong — and the documents that say it are written by hand, with
# nothing to notice when the count changes.
swap docs/patterns/everything-is-a-command.md \
    'The registry currently holds 23 commands across 7 areas' \
    'The registry currently holds 24 commands across 7 areas'
expect_catch "prose: the number of commands is not the registry's number" \
    "cargo xtask lists"

# A panic in the drawing code. §76: the render path holds the session's lock
# for the length of a frame, so a panic in a widget happens while the lock is
# held — and the panic hook runs on the panicking thread and uses `try_lock`,
# so it could not write a report even if it tried. Measured: a panic while the
# session was locked wrote *no report at all*, and the hook's own comment used
# to name that case as the one it existed for. The render path catches it now,
# because that is the one place that can still see the session.
swap apps/craftkit/src/main.rs \
    '    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| draw(session)));' \
    '    let outcome: Result<(), Box<dyn std::any::Any + Send>> = Ok(());
    draw(session);'
expect_catch "desktop: a panic in the drawing code writes no report" \
    "cargo test -p craftkit --profile ci a_panic_in_the_drawing_code_writes_a_report"

# A quoted struct that is not the one in the source. §75: the crash recipe
# quoted the whole of `CrashReport`, §72 changed two of its field types, and
# the recipe — the document a person reads when their application crashed —
# went on describing the old shape. A block marked `exact` is a claim that it
# is the source, and the gate holds it to that.
swap docs/recipes/fix-a-crash.md \
    '    pub design: craftkit_model::Document,   // the document at the moment of the crash' \
    '    pub design: serde_json::Value,   // the document at the moment of the crash'
expect_catch "prose: an exact quote of a struct is not the struct in the source" \
    "cargo xtask lists"

# A limit in the model's table that nothing enforces. §74: `MAX_EDGES` was in
# the table for years and bounded "the reverse index" — and there is no
# reverse index: `Document::index_of` is a scan, bounded by `MAX_NODES`. Two
# documents quoted the limit as one of the tree's hard limits, and an agent
# told "`MAX_EDGES` is 20 000" caps something with it and believes it is
# protected. A limit nobody enforces is a wish.
swap crates/model/src/lib.rs \
    '    /// The longest node name, in bytes.
    pub const MAX_NAME: usize = 255;' \
    '    /// The longest node name, in bytes.
    pub const MAX_NAME: usize = 255;
    /// Edges the reverse index may hold.
    pub const MAX_EDGES: usize = 20_000;'
expect_catch "model: the limits table grows a limit nobody enforces" \
    "cargo xtask lists"

# The undo stack's memory budget. Both halves of it: the loop that drops the
# oldest steps, and the check that refuses a step that does not fit.
swap crates/engine/src/history.rs \
    '        while self.bytes + bytes > MAX_HISTORY_BYTES {' \
    '        while false {'
swap crates/engine/src/history.rs \
    '        if self.bytes + bytes > MAX_HISTORY_BYTES {' \
    '        if false {'
expect_catch "engine: the undo stack ignores its memory budget" \
    "cargo test -p craftkit-engine --profile ci the_history_keeps_its_memory_within_a_budget"

# `run` and `snapshot` take a script, and the shape was documented only in this
# file's module comment — which nobody reading `--help`, or an error message,
# ever sees. A tool built to be driven by an agent, whose one required input
# format was invisible from the tool.
#
# The mutation removes the heading rather than rewording it. A first attempt
# replaced `SCRIPT FORMAT` with `SCRIPT FORMAT IS A SECRET`, and it did not
# bite: the test's own guard is a substring check, so it happily matched the
# mangled text. That is §1's substring disease, caught the day it was written,
# and it is why the mutation changes what the guard looks for.
swap apps/craftkit-cli/src/main.rs \
    'SCRIPT FORMAT (`run` and `snapshot`):' \
    'SEE THE MODULE DOC:'
expect_catch "cli: the help stops showing the script format it requires" \
    "cargo test -p craftkit-cli --profile ci usage_tests"

# The web page's save button said `saved {text} bytes under …` where it meant
# `text.len()`. Nothing was wrong in a test, because no test read the note; it
# took a real click in a real browser to see a 7 KB plan fill the panel and push
# the toolbar off the window.
swap apps/craftkit-web/src/main.rs \
    '    format!("saved {} bytes under “{STASH_KEY}”", text.len())' \
    '    format!("saved {text} bytes under “{STASH_KEY}”")'
expect_catch "web: the save note carries the whole plan instead of its size" \
    "cargo test -p craftkit-web --profile ci stash_note"

# `ui.click` parsed to `Method::Ui` and was then never read: the host took the
# action from `params.action`, which defaulted to `render`. The request an agent
# is told to send drew a 1180×720 PNG and answered `rendered 1180x720`, ok: true,
# having clicked nothing. Put the action back in front of the method name.
swap crates/control/src/protocol.rs \
    '            "ui.click" => Some(UiAction::Click),' \
    '            "ui.click" => None,'
expect_catch "control: the ui.click method name stops deciding what a ui request does" \
    "cargo test -p craftkit-control --profile ci ui_action"

# The click reply said `clicked \`toolbar.node.duplicate\`` — three in a row
# against a live app, with the document byte-identical afterwards. A click is a
# hit test; the sentence must not claim the other thing.
swap apps/craftkit/src/main.rs \
    '        Some(name) => format!(
            "`{name}` is at that point; a click reports where it landed and does not activate \
             anything — run it with `engine.execute`"
        ),' \
    '        Some(name) => format!("clicked `{name}`"),'
expect_catch "desktop: the click reply claims to have clicked" \
    "cargo test -p craftkit --profile ci click"

# The playbook's click example was `{"method":"ui.render","params":{"action":
# "click"}}` for months. It worked by accident, and the docs taught it twice.
swap docs/agent-playbook.md \
    '{"id":3,"method":"ui.click","params":{"name":"nodes.row.1.del"}}' \
    '{"id":3,"method":"ui.render","params":{"action":"click","name":"nodes.row.1.del"}}'
expect_catch "prose: a control-channel example that could not be sent" \
    "cargo xtask lists"

# `AGENTS.md` told every agent that `docs/control-protocol.md` has the wire
# format. That file has never existed, and `citation_drift` only looks at `.rs`.
swap AGENTS.md \
    '`docs/agent-playbook.md` has the wire format' \
    '`docs/control-protocol.md` has the wire format'
expect_catch "prose: a document points at a document that is not in the tree" \
    "cargo xtask lists"

# A citation that names a line but not a file: 13 of the 24 in the tree were
# wrong, and `citation_drift` could not see one of them, because `name.rs:12`
# has a name in it and `:12` does not.
swap docs/recipes/add-an-icon.md \
    '`an_unknown_icon_is_reported_not_drawn` asserts' \
    '`an_unknown_icon_is_reported_not_drawn` (`:300`) asserts'
expect_catch "prose: a citation names a line and no file" \
    "cargo xtask lists"

# `AGENTS.md` says "a panel never calls the evaluator", and nothing held it:
# the shell depends on craftkit-eval for two of its types, and could have
# called the evaluator itself - which would put a second road to the answer
# on the screen and cost every frame an evaluation of the whole document.
swap crates/ui/src/shell.rs \
    'pub fn node_rows(document: &craftkit_model::Document, plan: Option<&craftkit_eval::Plan>) -> Vec<NodeRow> {' \
    'pub fn node_rows(document: &craftkit_model::Document, plan: Option<&craftkit_eval::Plan>) -> Vec<NodeRow> {
    let _ = craftkit_eval::evaluate_once(document, craftkit_core::Tick::ZERO);'
expect_catch "ui: the shell evaluates the document" "cargo xtask lists"

# `AGENTS.md` told every agent that any new canvas calls `navigation::handle`.
# This tree has no canvas, no camera and no `navigation` module: the rule named
# a module that was never built here, and nothing checked it - the symbol
# claims gate needs a `*.rs` path beside the symbol, and the document-links
# gate needs a `.md`.
swap AGENTS.md \
    '  them, with its own caps — and every canvas must go through that module' \
    '  them, with its own caps — and every canvas calls `navigation::handle` instead'
expect_catch "prose: a module path that names a module this tree does not have" \
    "cargo xtask lists"

# The README has a second copy of the layer table, in a different shape from
# the one in docs/architecture.md. Only the architecture one was checked, so the
# README's was decoration.
swap README.md \
    'crates/kernel   L2   the numeric kernel, behind the panic guard' \
    'crates/kernel   L3   the numeric kernel, behind the panic guard'
expect_catch "the README's layer table drifts from the code" \
    "cargo xtask lists"

# A subcommand the parser accepts and the help never mentions: an agent reads
# the help, so a subcommand that is not in it does not exist as far as anybody
# driving this tool is concerned. The help and the parser are two spellings of
# one list, and nothing kept them in step.
swap apps/craftkit-cli/src/args.rs \
    '        "mcp" => {' \
    '        "export" => Err(UsageError::UnknownSubcommand("export".into())),
        "mcp" => {'
expect_catch "cli: the help stops listing every subcommand the parser accepts" \
    "cargo test -p craftkit-cli --profile ci usage_tests"

# A recipe that named a test that does not exist, in the same sentence as the
# right one: `every_command_has_an_icon_it_can_draw` for
# `every_command_has_an_icon_this_set_can_draw`.
swap docs/recipes/add-an-icon.md \
    '(in `crates/ui/src/icons.rs`) fails the moment' \
    '(the `every_command_has_an_icon_it_can_draw` test in `crates/ui/src/icons.rs`) fails the moment'
expect_catch "prose: a symbol claimed to be in a file is not in it" \
    "cargo xtask lists"

# A command that says it is read-only and writes to the document anyway. The
# `mutates` flag is what an agent reads before using a command as a probe.
swap crates/engine/src/commands.rs \
    '    run: |s, p| {
        let rate = s.document().meta.frame_rate();
        let to = match opt_time(p, "seconds", rate)? {' \
    '    run: |s, p| {
        s.doc_mut().meta.name = "written by a read-only command".to_string();
        let rate = s.document().meta.frame_rate();
        let to = match opt_time(p, "seconds", rate)? {'
expect_catch "a read-only command writes to the document" \
    "cargo test -p craftkit-engine --profile ci read_only"

# A command that changes the document in a way the format does not carry loses
# its work when the file is closed — quietly, because the command reported
# success. `muted` is the smallest field that proves it.
swap crates/model/src/document.rs \
    '    /// Whether the evaluator skips it.
    pub muted: bool,' \
    '    /// Whether the evaluator skips it.
    #[serde(skip)]
    pub muted: bool,'
expect_catch "the format stops carrying a command's effect" \
    "cargo test -p craftkit-cli --profile ci --test command_effects_survive_the_file"

# The rollback is four statements, and two of them were pinned by no test at
# all: deleting either of these left every test in the workspace green. The
# plan the window draws is the plan the restore leaves behind, and a document
# that arrives from outside — the app opening a file — is not a command, so
# `execute`'s success path never recomputes it. This is the §21 bug in a new
# place: the engine says one thing and the screen says another.
swap crates/engine/src/session.rs \
    '        self.recompute();
        self.dirty = dirty;' \
    '        self.dirty = dirty;'
expect_catch "engine: a rollback does not re-evaluate the plan" \
    "cargo test -p craftkit-engine --profile ci re_evaluates_the_plan"

# The other one: a rollback that leaves a save prompt for changes it rolled
# back. No command sets the flag today, so this is a precaution — and a
# precaution nobody checks is a comment.
swap crates/engine/src/session.rs \
    '        self.dirty = dirty;' \
    '        let _ = dirty;'
expect_catch "engine: a rollback does not put the dirty flag back" \
    "cargo test -p craftkit-engine --profile ci dirty_flag"

# The safety limits, as the documents state them. These are the numbers that
# bound a hostile file, and they are written down in three places so that an
# agent coding against them uses the right ones. All twenty of the claims
# were true — by luck, because nothing compared them with the code.
swap docs/architecture.md \
    '`MAX_DEPTH` 32' \
    '`MAX_DEPTH` 33'
expect_catch "prose: a limit in prose is not the constant's value" \
    "cargo xtask lists"

# The same limit in a table, where the value and the constant are in
# different cells.
swap docs/patterns/clone-before-edit.md \
    '| Undo entries kept | 128 |' \
    '| Undo entries kept | 127 |'
expect_catch "prose: a limit in a table cell is not the constant's value" \
    "cargo xtask lists"

# The "did this command actually change anything?" check. The engine compares
# the document with the snapshot it took before the command, and a command
# that declares `mutates: true` and writes nothing is a bug.
#
# §93 changed the shape of the check — the command now says whether it
# changed anything, because the comparison cannot tell "the command forgot to
# write" from "the command wrote the value a field already held" — and this
# anchor moved with it. The check is still the one that catches a command
# that forgot to write: the default of `Outcome::changed` is "it did", so a
# command that says nothing is caught. The gate here is the old test; §93's
# own case removes the same check and is caught by the new one.
swap crates/engine/src/session.rs \
    '                    if !changed && outcome.changed {' \
    '                    if false {'
expect_catch "engine: a command that changes nothing is allowed" \
    "cargo test -p craftkit-engine --profile ci says_it_changes_the_document"

# The rollback in that branch, which is what puts the dirty flag back.
swap crates/engine/src/session.rs \
    '                        self.restore(snapshot, before_dirty);
                        let err = EngineError::Internal {
                            id: id.to_string(),
                            detail: "the command declared that it mutates the document but changed nothing"' \
    '                        let err = EngineError::Internal {
                            id: id.to_string(),
                            detail: "the command declared that it mutates the document but changed nothing"'
expect_catch "engine: a refused no-op leaves the dirty flag set" \
    "cargo test -p craftkit-engine --profile ci marks_the_project_dirty"

# The refusal path's log entry. The log test used an unknown id, which fails
# in the lookup and never reaches the rollback.
swap crates/engine/src/session.rs \
    '                self.log.record(id, false, &e.to_string());
' ''
expect_catch "engine: a refused command is not logged" \
    "cargo test -p craftkit-engine --profile ci refused_command_is_logged"

# The panic path's log entry, for the same reason.
swap crates/engine/src/session.rs \
    '                self.log.record(id, false, &err.to_string());
                Err(err)' \
    '                Err(err)'
expect_catch "engine: a panicking command is not logged" \
    "cargo test -p craftkit-engine --profile ci panicking_command_is_logged"

# The pending-change mechanism. `FILES` has been empty since version 1, so
# `apply` had never run a step and nothing tested the order the module
# promises: "step functions run in order, each seeing what the previous one
# produced". The first real format change would have been the first
# execution.
swap crates/io/src/format/pending.rs \
    '    for step in steps {' \
    '    for step in steps.iter().rev() {'
expect_catch "io: the pending steps do not run in order" \
    "cargo test -p craftkit-io --profile ci steps_run"

# A citation that names a test, and the line it points at. Four of them
# pointed 9, 52, 100 and 147 lines away from the test they name.
swap docs/recipes/change-the-file-format.md \
    '`the_files_on_disk_and_the_table_agree` (both in
`crates/io/src/format/pending.rs`).' \
    '`the_files_on_disk_and_the_table_agree` (both in
`crates/io/src/format/pending.rs:89`).'
expect_catch "prose: a citation that names a test points at the wrong line" \
    "cargo xtask lists"

# ------------------------------------------------------------------- prose ---
# A pattern document that quotes a function and calls the block `exact` is
# claiming to be the source, word for word. The block used to say "the four
# restore statements", and said "one", "two" and "three" in three other
# places: a number in a sentence is not a citation, and `citation_drift` only
# reads `file.rs:12`. Quoting the code instead of counting it is what makes
# the claim checkable, and this is the check.
swap crates/engine/src/session.rs \
    '        self.dirty = dirty;' \
    '        self.dirty = true;'
expect_catch "prose: the function a document quotes is not the function in the file" \
    "cargo xtask lists"

# The other direction: the document loses a line the function has. A quote
# that has quietly been edited is exactly what the marker is there to catch.
#
# §115 added a line to `restore` — the playhead follows the document in — so
# this anchor moved with it. The line removed here is still one the function
# has: `self.recompute();`.
swap docs/patterns/clone-before-edit.md \
    '    self.clamp_playhead_into_range();
    self.recompute();' \
    '    self.clamp_playhead_into_range();'
expect_catch "prose: a quoted function loses a line in the document" \
    "cargo xtask lists"

# The line count a document gives a file. `crates/io/src/format/pending.rs`
# was "110 lines" in the playbook and "98 lines" in a recipe, and it is 174:
# two documents disagreeing about one file, and both wrong.
swap docs/recipes/change-the-file-format.md \
    'Read `crates/io/src/format/pending.rs` top to bottom. It is the rule.' \
    'Read `crates/io/src/format/pending.rs` top to bottom. It is 999 lines and it is the rule.'
expect_catch "prose: a line count in prose is not the file's length" \
    "cargo xtask lists"

# The other form: the line where a file's tests begin.
#
# The playbook used to name the line itself ("the tests start at line 960"),
# and every change to the file moved it. It now names the *place* — the tests
# module at the end of the file — so the claim cannot rot; and the gate still
# catches a document that names a line, because the mutation below is exactly
# that: a document that names a line, and names it wrongly.
swap docs/agent-playbook.md \
    'its tests are at the end of the file' \
    'the tests start at line 900'
expect_catch "prose: the line a document names is not where the tests start" \
    "cargo xtask lists"

# Code that prose attributes to a file. The crash recipe used to say "the two
# `self.doc = snapshot;` lines in `crates/engine/src/session.rs`", and §40
# deleted those lines: the restore is one function now. Nothing checked code
# quoted in prose, so the recipe kept quoting code that no longer exists.
swap docs/recipes/fix-a-crash.md \
    'the `Session::restore` function in `crates/engine/src/session.rs`' \
    'the two `self.doc = snapshot;` lines in `crates/engine/src/session.rs`'
expect_catch "prose: a statement prose attributes to a file is not a line of it" \
    "cargo xtask lists"

# The other form: a method that is not in the file it is attributed to.
swap docs/recipes/fix-a-crash.md \
    'the `Session::restore` function in `crates/engine/src/session.rs`' \
    'the `Session::restore_snapshot` function in `crates/engine/src/session.rs`'
expect_catch "prose: a method prose attributes to a file is not in it" \
    "cargo xtask lists"

# The token file is the promise that a port answers to a token, so the instance
# that holds the port is the only one that may write it. §54.
#
# The mutation publishes *before* the bind, which is what the app used to do:
# a second instance, which cannot bind, replaced the running instance's token
# with its own, and every request to the running app was refused with "a bad
# token".
swap crates/control/src/server.rs \
    '    let (port, server) = serve(session, options)?;' \
    '    let _ = write_token_file(dir.as_deref(), requested, &token)?;
    let (port, server) = serve(session, options)?;'
expect_catch "control: a second instance takes the first one's token file" \
    "cargo test -p craftkit-control --profile ci a_second_instance_does_not_take_the_first_ones_token_file"

# The live-window check measures the window of *this* application. It used to
# take the first window named "craftkit", which with a second instance running
# is the other instance's window: the check then reported "the engine changed
# and the window did not", a confident diagnosis of a bug that was not there.
# The pid is what tells the windows apart. §56.
swap scripts/live-window-check.sh \
    '        if [ -n "${pids[$i]}" ] && [ "${pids[$i]}" = "$want" ]; then' \
    '        if [ -n "${pids[$i]}" ]; then'
expect_catch "live-window: the check stops telling one instance from another" \
    "./scripts/live-window-check.sh --self-test"

# A crash report holds the user's design — the document as it was and every
# command that was run against it — and it was written world-readable: the
# folder 0775 and the files 0664, both measured. §57.
swap crates/engine/src/crash.rs \
    '    restrict(&staging, true);' \
    '    let _ = &staging;'
expect_catch "engine: the crash report's folder is left readable by other users" \
    "cargo test -p craftkit-engine --profile ci a_crash_report_is_not_readable_by_other_users"

# The files are created private rather than created open and locked down
# afterwards, so the design is never on disk in a file another user can read.
swap crates/engine/src/crash.rs \
    '        .mode(0o600)' \
    '        .mode(0o644)'
expect_catch "engine: the crash report's design is written readable by other users" \
    "cargo test -p craftkit-engine --profile ci a_crash_report_is_not_readable_by_other_users"

# A folder that was already there belongs to whoever made it. `CRAFTKIT_CRASH_DIR`
# can name one that other people use too.
swap crates/engine/src/crash.rs \
    '    if base_is_ours {
        restrict(base, true);
    }' \
    '    let _ = base_is_ours;
    restrict(base, true);'
expect_catch "engine: a crash folder the tool did not create is made private anyway" \
    "cargo test -p craftkit-engine --profile ci a_crash_folder_that_was_already_there_keeps_its_permissions"

# The crash report's id is what says when the crash happened, and what sorts
# the reports into order. It was a number with no date in it, restarting every
# 1 157 days, and not the format `CrashReport::id` documents. §58.
swap crates/engine/src/crash.rs \
    '    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}-{:04x}", (nanos >> 8) & 0xffff)' \
    '    let _ = (year, month, day, hour, minute, second);
    format!("{:08}-{:04x}", unix % 100_000_000, (nanos >> 8) & 0xffff)'
expect_catch "engine: the crash report's id stops carrying a date" \
    "cargo test -p craftkit-engine --profile ci the_id_is_the_documented_format"

# The date arithmetic, one day out.
swap crates/engine/src/crash.rs \
    '    let z = days + 719_468;' \
    '    let z = days + 719_467;'
expect_catch "engine: the crash report's date is a day out" \
    "cargo test -p craftkit-engine --profile ci the_date_is_right_for_a_known_day"

# A report is the only evidence a crash happened. Two crashes in the same
# second landed on the same id, and the second overwrote the first.
swap crates/engine/src/crash.rs \
    '    while dir.exists() {' \
    '    while false && dir.exists() {'
expect_catch "engine: a second crash destroys the first crash's report" \
    "cargo test -p craftkit-engine --profile ci a_second_crash_does_not_destroy_the_first"

# The frame size was capped per side and the area was not, so 8192 × 8192 was
# allowed: 67 million pixels, 268 MB of pixels, 301 MB of resident memory, and
# a 14 789 463-byte reply — while the session was locked for the duration.
# §59.
swap crates/ui/src/raster.rs \
    '    let pixels = width.saturating_mul(height);
    if pixels > MAX_PIXELS {' \
    '    let pixels = width.saturating_mul(height);
    if false && pixels > MAX_PIXELS {'
expect_catch "ui: a frame's area stops being bounded" \
    "cargo test -p craftkit-ui --profile ci a_frame_that_is_too_big_in_pixels_is_refused"

# The model's tick range was wider than a JSON number's, so saturating a
# hostile time produced a value no reply could hold — and `json!` failed
# inside the macro, where the lint that forbids `unwrap` cannot see it. §60.
swap crates/core/src/tick.rs \
    '    pub const MAX: Tick = Tick(u64::MAX as i128);' \
    '    pub const MAX: Tick = Tick(i128::MAX / 4);'
expect_catch "core: a tick can be wider than a reply can carry" \
    "cargo test -p craftkit-engine --profile ci a_hostile_time_does_not_panic_the_command_that_reports_it"

# The reader has to refuse a file whose range no reply could carry.
#
# The swap is repeated for the second gate on purpose: `expect_catch` restores
# the tree after every case, so one swap serves exactly one gate. Writing the
# two gates under one swap made the second one run against an unmutated tree —
# and the suite said so, which is the whole point of it.
swap crates/model/src/document.rs \
    '        if self.meta.start < craftkit_core::Tick::MIN || self.meta.end > craftkit_core::Tick::MAX {' \
    '        if false && (self.meta.start < craftkit_core::Tick::MIN || self.meta.end > craftkit_core::Tick::MAX) {'
expect_catch "model: a document whose range no reply can carry is accepted" \
    "cargo test -p craftkit-model --profile ci a_document_whose_range_is_outside_what_a_time_can_be_is_refused"

swap crates/model/src/document.rs \
    '        if self.meta.start < craftkit_core::Tick::MIN || self.meta.end > craftkit_core::Tick::MAX {' \
    '        if false && (self.meta.start < craftkit_core::Tick::MIN || self.meta.end > craftkit_core::Tick::MAX) {'
expect_catch "io: a file whose range is too far out is read back" \
    "cargo test -p craftkit-io --profile ci a_file_whose_range_is_too_far_out_is_refused"

# The token file is private because the *file* is private. The old code kept
# it private by making the directory 0700 instead, and a directory is not the
# tool's to make private unless the tool made it: pointing
# `CRAFTKIT_CONTROL_DIR` at a shared directory turned it from 0755 into 0700
# behind the user's back, and dropped the setgid bit on the way. §55.
swap crates/control/src/token.rs \
    '        if ours {
            restrict(parent, true).map_err(|e| format!("could not protect {}: {e}", parent.display()))?;
        }' \
    '        let _ = ours;
        restrict(parent, true).map_err(|e| format!("could not protect {}: {e}", parent.display()))?;'
expect_catch "control: a directory the tool did not create is made private anyway" \
    "cargo test -p craftkit-control --profile ci a_directory_that_was_already_there_keeps_its_permissions"

# The file is created 0600, so the secret is never on disk in a file another
# user can read — not even for the length of the chmod that used to follow
# the write.
swap crates/control/src/token.rs \
    '        .mode(0o600)' \
    '        .mode(0o644)'
expect_catch "control: the token file is created readable by other users" \
    "cargo test -p craftkit-control --profile ci the_token_file_is_private_in_a_shared_directory"

# The tool's own directory is still made private. The file's own mode is what
# keeps the secret, but the directory is the tool's to protect.
swap crates/control/src/token.rs \
    '    let mode = if dir { 0o700 } else { 0o600 };' \
    '    let mode = if dir { 0o755 } else { 0o600 };'
expect_catch "control: the tool's own directory is left readable by other users" \
    "cargo test -p craftkit-control --profile ci a_directory_the_tool_creates_is_private"

# The token is written into a fresh file and moved into place, so a token file
# left over from a previous launch is replaced rather than kept. A file that
# survives its launch names a token the app does not answer to — the same
# family as §54, where the file held a token and the port did not.
swap crates/control/src/token.rs \
    '    std::fs::rename(temp, path).map_err(|e| {' \
    '    if path.exists() {
        let _ = std::fs::remove_file(temp);
        return Ok(());
    }
    std::fs::rename(temp, path).map_err(|e| {'
expect_catch "control: a token file that already exists is not replaced" \
    "cargo test -p craftkit-control --profile ci a_token_file_left_over_from_a_previous_launch_is_replaced"

# The evaluator's cache is content-addressed, which is what makes clearing it
# on a restore belt and braces rather than a correctness requirement. Stop
# hashing the node's properties and the cache starts serving states that
# belong to a document that no longer exists.
#
# The fingerprint moved to the model in §53 — a structural hash costs a tenth
# of what hashing the serialised form did — so the mutation moved with it.
swap crates/model/src/fingerprint.rs \
    '        h.write_u64(self.nodes.len() as u64);
        for node in &self.nodes {
            match node {
                PropNode::Prop(p) => {
                    h.write_u8(0);
                    p.hash_into(h);
                }
                PropNode::Group(g) => {
                    h.write_u8(1);
                    g.hash_into(h);
                }
            }
        }' \
    '        h.write_u64(self.nodes.len() as u64);'
expect_catch "eval: the fingerprint stops looking at the node's properties" \
    "cargo test -p craftkit-eval --profile ci"

# The same break, one field over: the node's *name* stops being part of its
# fingerprint. Every test in the evaluator still passes — they all change
# values and positions — and only the engine's own test sees it, because it
# is the only one that renames a node and then puts the document back.
swap crates/model/src/fingerprint.rs \
    '        h.write_str(&self.name);' \
    '        // MUTATED: the name is no longer part of the fingerprint.'
expect_catch "engine: a restore is served a state from the document it replaced" \
    "cargo test -p craftkit-engine --profile ci a_restore_cannot_serve"

# The keyframes. A hand-written hash is most likely to forget the part that
# is hardest to see, and nothing in the tree checked it: the other cache
# tests change a property's value or move a node, and none of them changed a
# *keyframe*. §53.
swap crates/model/src/fingerprint.rs \
    '        self.keys.hash_into(h);' \
    '        // MUTATED: the keyframes are no longer part of the fingerprint.'
expect_catch "eval: the fingerprint stops looking at the keyframes" \
    "cargo test -p craftkit-eval --profile ci a_node_whose_keyframes_changed_is_not_served_from_the_cache"

# -------------------------------------------------------------------- ci ----
# `ci` used to run the cheap gates from `PHASES` and then call the tests and the
# wasm check separately. Deleting that one call left it printing "ci green — …,
# tests, wasm" and exiting 0, because the closing line was a string literal
# rather than a record of what had run. Both halves are checked here.
swap xtask/src/ci.rs '    Phase { name: "tests", run: tests },
    Phase { name: "fuzz", run: fuzz::command },
    Phase { name: "wasm", run: wasm::run },
' ''
expect_catch "ci: the tests, fuzz and wasm phases vanish from the run" \
    "cargo test -p xtask --profile ci the_phases_are_the_documented_ones_and_come_first"

# The closing line itself: naming a phase that did not run is the whole defect,
# so put it back the way it was and check the summary catches it.
swap xtask/src/ci.rs '        crate::Outcome::Ran => ran.push(name),
        crate::Outcome::Skipped => skipped.push(name),' \
    '        crate::Outcome::Ran => ran.push(name),
        crate::Outcome::Skipped => ran.push(name),'
expect_catch "ci: a skipped phase is counted as one that ran" \
    "cargo test -p xtask --profile ci a_phase_that_could_not_run_is_not_counted_as_one_that_ran"

# ------------------------------------------------------------- cleanroom ----
# The clean-room gate: the tree may not carry another product's material.
#
# Three mutations, each putting the rule back the way it was before the gate
# existed -- the rule was a sentence in AGENTS.md and no tool could check it.
# §168.
#
# The first is the one that matters: a gate that reads no table checks nothing
# and passes everything, and it would still say "ok" on a tree full of copies.
swap xtask/src/cleanroom.rs \
    '    let products = prohibited(&text)?;' \
    '    let products: Vec<Prohibited> = Vec::new();'
expect_catch "cleanroom: the rule's table is read as empty, so the gate checks nothing" \
    "cargo test -p xtask --profile ci a_product_name_in_a_source_file_is_reported_with_its_line"

# The second: a product's name in a file is written however the writer felt
# like writing it, and a name in lower case is the product as much as the same
# name in capitals is. The name here is invented: the clean-room rule says a
# product's name may not appear outside prose, and a shell script is not prose.
swap xtask/src/cleanroom.rs \
    '    let haystack = haystack.to_lowercase();
    let name = name.to_lowercase();' \
    '    let haystack = haystack.to_string();
    let name = name.to_string();'
expect_catch "cleanroom: a product's name written in lower case is missed" \
    "cargo test -p xtask --profile ci the_case_of_a_name_does_not_matter"

# The third: a two-word product's name cannot hold a space in a Rust file
# name, so the path is checked in three forms. Lose two of them and a file
# called "zorbix-export.rs" is not a violation any more.
swap xtask/src/cleanroom.rs \
    '            if path_mentions(&shown, &product.name) {' \
    '            if mentions(&shown, &product.name) {'
expect_catch "cleanroom: a two-word product's name in a file name is missed" \
    "cargo test -p xtask --profile ci a_product_name_in_a_file_name_is_reported"

# ------------------------------------------------------------------- web ----
# The *element's* id, not merely the first mention of the name. An earlier
# mutation renamed only the CSS selector, changed nothing the browser cares
# about, and taught us that a substring search can be fooled by the name
# surviving elsewhere in the file.
swap apps/craftkit-web/index.html '<canvas id="craftkit-canvas"' '<canvas id="c-canvas"'
expect_catch "web: the canvas element's id is renamed (the name survives elsewhere)" \
    "cargo xtask web"

swap apps/craftkit-web/index.html '#craftkit-canvas {' '#not-the-canvas {'
expect_catch "web: the stylesheet stops sizing the canvas" "cargo xtask web"

swap apps/craftkit-web/sw.js 'if (url.origin !== self.location.origin) return;' '// removed'
expect_catch "web: the service worker would serve another origin" "cargo xtask web"

swap apps/craftkit-web/sw.js 'if (request.method !== "GET") return;' '// removed'
expect_catch "web: the service worker would answer a non-GET" "cargo xtask web"

# The bug a real browser found and every compile check was green through: the
# page imported `craftkit_web.js`, `wasm-bindgen` wrote `craftkit-web.js`, and
# the load was a 404. The name is one constant now; move it and the page's
# import no longer matches what the bundler is asked to emit.
swap apps/craftkit-web/index.html 'craftkit_web.js' 'craftkit-web.js'
expect_catch "web: the page imports a bundle name the bundler does not emit" "cargo xtask web"

swap xtask/src/wasm.rs 'pub const BUNDLE_NAME: &str = "craftkit_web";' \
    'pub const BUNDLE_NAME: &str = "craftkit-web";'
expect_catch "web: the bundler is told to emit a name the page does not import" "cargo xtask web"

# The failure box a broken page shows is the only instruction its reader ever
# sees, and it named `cargo xtask wasm` — the command that checks and writes
# nothing. §11's fix corrected the authoring comments in the same file and left
# this copy, which is exactly the two-spellings bug §11 is about.
swap apps/craftkit-web/index.html \
    '1. The bundle is missing.        Run:  cargo xtask bundle' \
    '1. The bundle is missing.        Run:  cargo xtask wasm'
expect_catch "web: the failure box tells the reader to run a command that builds nothing" \
    "cargo xtask web"

# ------------------------------------------------------------------ wasm ----
# The boundary this gate enforces is written down in xtask/src/wasm.rs.
swap crates/model/src/lib.rs 'pub mod document;' 'pub mod document;

/// Looks harmless; is not.
pub fn host_only() -> u32 {
    std::env::var("HOME").map(|_| 1u32).unwrap_or(0)
}'
expect_miss "wasm: a library crate calls std::env in a lower layer" \
    "cargo xtask wasm" \
    "host-only std calls compile for wasm32 and fail at run time instead, so compiling cannot find them"

# The class this gate *does* catch: a cfg branch that stopped compiling. Nothing
# else builds a wasm32 branch until somebody opens the page.
swap crates/model/src/lib.rs 'pub mod document;' 'pub mod document;

/// Only compiled in a browser build, and broken.
#[cfg(target_arch = "wasm32")]
pub fn browser_only() -> u32 {
    let n: u32 = "not a number";
    n
}'
expect_catch "wasm: a cfg(wasm32) branch stopped compiling" "cargo xtask wasm" "yes"

# `wasm` is a compile check and writes nothing, so it is green on a machine
# where the page's JavaScript cannot be produced at all. The phase now says so.
#
# Mutating the sentence, not the printing: a check that grepped `cargo xtask
# wasm`'s output would pass on a machine with a matching wasm-bindgen CLI and
# fail on one without, so it would be reporting the checkout rather than the
# tree. `post_build_note` is pure for the same reason.
swap xtask/src/wasm.rs \
    'blocker.map(|reason| {' \
    'None::<&str>.map(|reason| {'
expect_catch "wasm: a green compile check stops saying the bundle is unbuilt" \
    "cargo test -p xtask --profile ci post_build_note"

# `doctor` and `cargo xtask bundle` each answered "can this machine produce the
# bundle?" with their own copy of the rule, and the copies disagreed: when the
# wasm-bindgen crate version could not be read out of Cargo.lock, `bundle` read
# that as "no blocker" and carried on with a CLI it had checked against nothing,
# while `doctor` filed it as `info`. They now share `bundler_status`.
#
# This mutation puts the old, unsafe reading back. Nothing else notices: the
# lockfile here parses, so only the test knows the difference.
swap xtask/src/doctor.rs \
    '    let Some(want) = crate_version else {
        return Bundler::Blocked(format!(
            "the wasm-bindgen crate version could not be read from Cargo.lock, so the CLI {cli} \
             cannot be checked against it"
        ));
    };' \
    '    let Some(want) = crate_version else {
        return Bundler::Ready;
    };'
expect_catch "wasm: an unreadable crate version counts as a working bundler again" \
    "cargo test -p xtask --profile ci bundler_status"

# The counter that reports recovered kernel panics was a `thread_local!` cell,
# so it counted the recoveries of the thread that *asked* — and the thread that
# asks is the control channel's, while the kernel's work happens on the UI
# thread. The counter was structurally incapable of reporting the thing it
# exists to report. §62.
#
# The swap replaces the whole block: the storage, the increment and the read.
# One swap, one gate — so the engine's own test, which would also fail under
# this swap, is given its own swap below rather than a second gate here.
swap crates/kernel/src/lib.rs \
    'static RECOVERED: AtomicU64 = AtomicU64::new(0);

/// Count one recovered panic.
///
/// Saturating rather than wrapping: the count is a lifetime statistic, and a
/// number that silently went back to zero would be read as "nothing has ever
/// gone wrong".
fn note_recovered() {
    let _ = RECOVERED.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v.saturating_add(1)));
}

/// How many panics this boundary has recovered since the process started.
///
/// The count is **process-wide**: a recovery on any thread is visible from any
/// other. See the note on [`RECOVERED`] for why it cannot be per-thread.
pub fn recovered_count() -> u64 {
    RECOVERED.load(Ordering::Relaxed)
}' \
    'thread_local! {
    static RECOVERED: Cell<u64> = const { Cell::new(0) };
}

/// Count one recovered panic.
fn note_recovered() {
    RECOVERED.with(|c| c.set(c.get().saturating_add(1)));
}

/// How many panics this boundary has recovered since the process started.
pub fn recovered_count() -> u64 {
    RECOVERED.with(|c| c.get())
}'
expect_catch "kernel: the recovery counter counts only the thread that asks" \
    "cargo test -p craftkit-kernel --profile ci a_recovery_on_one_thread_is_counted_on_another"

# The other end of the same wire: the session's statistics stop reading the
# counter at all. The counter can be perfect and the reply can still lie, and
# the reply is what an agent reads.
swap crates/engine/src/commands.rs \
    '                "kernel_recoveries": craftkit_kernel::recovered_count(),' \
    '                "kernel_recoveries": 0,'
expect_catch "engine: the statistics stop counting recovered kernel panics" \
    "cargo test -p craftkit-engine --profile ci the_session_statistics_count_a_recovery_that_happened_on_another_thread"

# The token comparison folded the length difference into a `u8` accumulator, so
# every length difference that was a multiple of 256 was lost — and the token
# followed by 256 NUL bytes was accepted as the token, by the real control
# channel. The accumulator is a `usize` now. §63.
swap crates/control/src/token.rs \
    '        let mut diff = a.len() ^ b.len();
        let n = a.len().max(b.len());
        for i in 0..n {
            // Wrapping so an index past the shorter slice contributes zero
            // rather than panicking — and the length is already folded into
            // `diff` above.
            diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
        }' \
    '        let mut diff = (a.len() ^ b.len()) as u8;
        let n = a.len().max(b.len());
        for i in 0..n {
            // Wrapping so an index past the shorter slice contributes zero
            // rather than panicking — and the length is already folded into
            // `diff` above.
            diff |= u8::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
        }'
expect_catch "control: the token comparison loses a length difference of 256" \
    "cargo test -p craftkit-control --profile ci a_token_with_trailing_nuls_is_not_a_token"

# The other half of the same mistake: the length is not folded in at all, so
# the comparison only ever looks at the bytes and never at how many there are.
swap crates/control/src/token.rs \
    '        let mut diff = a.len() ^ b.len();' \
    '        let mut diff = 0usize;'
expect_catch "control: the token comparison forgets that the lengths differ" \
    "cargo test -p craftkit-control --profile ci a_connection_with_a_padded_token_is_closed"

# The three readers of untrusted JSON had three unrelated limits — 1 MiB,
# 4 MiB and 32 MiB — and the largest of them bought a gigabyte of memory,
# because none of them was written down as what it was for. They are derived
# from one budget now. §64.
swap crates/mcp/src/rpc.rs \
    'pub const MAX_MESSAGE: usize = craftkit_core::MAX_JSON_TEXT_BYTES;' \
    'pub const MAX_MESSAGE: usize = 4 * 1024 * 1024;'
expect_catch "mcp: the message limit is a number of its own again" \
    "cargo test -p craftkit-mcp --profile ci the_message_limit_is_the_tree_s_json_limit"

swap apps/craftkit-cli/src/cli.rs \
    'pub const MAX_SCRIPT_BYTES: usize = craftkit_core::MAX_JSON_TEXT_BYTES;' \
    'pub const MAX_SCRIPT_BYTES: usize = 32 * 1024 * 1024;'
expect_catch "cli: the script limit is a number of its own again" \
    "cargo test -p craftkit-cli --profile ci the_script_limit_is_the_tree_s_json_limit"

# The number *between* two keyframes is not one of the numbers the tree
# checked, and an easing's overshoot is deliberate — so the interpolation has
# to clamp to the range the constructors hold to. It did not, and a source
# node took its level to 1 497 942 087. §65.
swap crates/model/src/keyframe.rs \
    '        (V::Num(x), V::Num(y)) => {
            V::Num((x + (y - x) * t).clamp(-craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE))
        }' \
    '        (V::Num(x), V::Num(y)) => V::Num(x + (y - x) * t),'
expect_catch "model: the interpolation stops clamping to the range" \
    "cargo test -p craftkit-model --profile ci an_overshooting_easing_stays_inside_the_range"

swap crates/model/src/keyframe.rs \
    '        (V::Num(x), V::Num(y)) => {
            V::Num((x + (y - x) * t).clamp(-craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE))
        }' \
    '        (V::Num(x), V::Num(y)) => V::Num(x + (y - x) * t),'
expect_catch "eval: a source takes the level out of range" \
    "cargo test -p craftkit-eval --profile ci an_animated_amount_never_takes_the_level_out_of_range"

# The widget names were `&'static str` made by `Box::leak`, and they are built
# every frame — the toolbar's, every node row's, every property editor's — so
# the process grew by a kilobyte a frame: 221 MB an hour at sixty frames a
# second. The names are owned now. §66.
#
# The leak comes back in the shape that only a measurement can see: the name
# leaks a copy of its bytes *and* owns another one, so the type still says
# `String` and everything still compiles. The gate measures the process's own
# resident pages, in a child process, because a leak is only visible against a
# baseline and a baseline is only worth anything in a process that has done
# nothing else.
swap crates/ui/src/automation.rs \
    '    pub fn toolbar(command_id: &str) -> Self {
        AutomationId(format!("toolbar.{command_id}"))
    }' \
    '    pub fn toolbar(command_id: &str) -> Self {
        AutomationId(Box::leak(format!("toolbar.{command_id}").into_boxed_str()).to_string())
    }'
expect_catch "ui: the widget names leak their bytes again" \
    "cargo test -p craftkit-ui --profile ci building_a_widget_name_does_not_leak_memory"

# The page and the service worker each hold a copy of the bundle's name. The
# page's copy was checked and the worker's was not, so the worker could
# precache a file that does not exist — and the gate said the two agreed. §67.
swap apps/craftkit-web/sw.js \
    '  "./dist/craftkit_web.js",
  "./dist/craftkit_web_bg.wasm",' \
    '  "./dist/craftkit_web_v2.js",
  "./dist/craftkit_web_v2_bg.wasm",'
expect_catch "web: the worker precaches a bundle that does not exist" \
    "cargo xtask web"

# The undo stack's budget counts the memory the two stacks hold together, and
# `push` clears the redo stack when a new edit arrives — the undone branch is
# unreachable, and its memory is gone. The budget kept counting it, so every
# edit after an undo added a phantom, and the budget trimmed the undo stack
# against it: on a full-size document, undo stopped working. §68.
swap crates/engine/src/history.rs \
    '    fn clear_redo(&mut self) {
        for entry in self.redo.drain(..) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
        }
    }' \
    '    fn clear_redo(&mut self) {
        self.redo.clear();
    }'
expect_catch "engine: the undo budget counts the memory it has just freed" \
    "cargo test -p craftkit-engine --profile ci the_budget_counts_the_memory_the_stacks_hold"

# The same mistake, seen as the person who meets it: the undo stack is trimmed
# against a phantom until it is empty, and undo stops doing anything.
swap crates/engine/src/history.rs \
    '    fn clear_redo(&mut self) {
        for entry in self.redo.drain(..) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
        }
    }' \
    '    fn clear_redo(&mut self) {
        self.redo.clear();
    }'
expect_catch "engine: undo stops working once the redo branches pile up" \
    "cargo test -p craftkit-engine --profile ci undo_still_works_after_many_edits_that_discard_a_redo_branch"

# The command log is a bounded ring of the last 256 commands, and the failure
# count was taken over the ring while its own doc said "since the session
# opened". A session that had run three hundred commands reported 256 of them
# and 255 of the failures. §69.
swap crates/engine/src/log.rs \
    '    pub fn failures(&self) -> usize {
        self.failures
    }' \
    '    pub fn failures(&self) -> usize {
        self.entries.iter().filter(|e| !e.ok).count()
    }'
expect_catch "engine: the failure counter counts only the commands in the ring" \
    "cargo test -p craftkit-engine --profile ci the_failure_count_is_every_failure_and_not_only_the_ones_in_the_ring"

# The other half of the same reply: the command count was the ring's length,
# so it stopped at 256 and stayed there.
swap crates/engine/src/commands.rs \
    '                "commands": s.log().total(),' \
    '                "commands": s.log().entries().len(),'
expect_catch "engine: the statistics count only the commands in the ring" \
    "cargo test -p craftkit-engine --profile ci the_session_statistics_count_every_command_and_not_only_the_last_two_hundred_and_fifty_six"

# The command trace holds the *parsed* parameters, and its budget was written
# in bytes of text: a parsed JSON value costs up to thirty-two times its text,
# and the loop that keeps the trace inside its budget keeps the last step, so
# one big step was kept whole — 2.7 MB of text, 15 MB of memory, for a budget
# of 256 KiB. §70.
swap crates/engine/src/session.rs \
    '        if step_bytes > crate::MAX_TRACE_BYTES {' \
    '        if step_bytes > usize::MAX {'
expect_catch "engine: a command too big for the trace is not refused" \
    "cargo test -p craftkit-engine --profile ci a_command_whose_parameters_do_not_fit_in_the_trace_is_refused"

# And the budget itself: the same number, but written out on its own instead
# of derived from the memory it is supposed to bound.
swap crates/engine/src/lib.rs \
    'pub const MAX_TRACE_BYTES: usize = MAX_TRACE_MEMORY_BYTES / craftkit_core::JSON_EXPANSION;' \
    'pub const MAX_TRACE_BYTES: usize = 256 * 1024;'
expect_catch "engine: the trace budget is a number of its own again" \
    "cargo test -p craftkit-engine --profile ci the_trace_budget_is_derived_from_the_memory_budget"

# The fingerprint forgets a field, and the guard that should have noticed
# drifts with it. §80: the keyframe's easing was in the hash but not in the
# hash's own test — the test changed a key's time, value and easing together,
# so the fingerprint moved whatever else was read. Removing the easing left
# **every test in the workspace green**, and the evaluator's cache went on
# serving the curve from before the edit.
swap crates/model/src/fingerprint.rs \
    '    hash_value(h, &key.value);
    hash_easing(h, &key.ease);' \
    '    hash_value(h, &key.value);'
expect_catch "model: the fingerprint forgets the keyframe's easing" \
    "cargo test -p craftkit-eval --profile ci a_changed_easing_is_not_served_from_the_cache"

# The other half of the same gap: the model grows a field, and the hash does
# not read it. Nothing in the tree noticed, because the guard was a list of
# the same fields the hash covers — a list that drifts with the thing it
# checks. The `lists` gate reads the tree, so it sees the field.
swap crates/model/src/property.rs \
    '    pub hint: PropHint,' \
    '    pub hint: PropHint,
    /// A field the hash does not read.
    pub unremembered: u8,'
expect_catch "model: the fingerprint forgets a field the model has" \
    "cargo xtask lists"

# A file is given only the changes it has not had. §81: the reader applied
# every pending change to every file, including the files it had just
# written, so a file did not read back as it was written — and the round-trip
# tests caught it the moment a real pending change existed, which is a day
# that had not come.
swap crates/io/src/format.rs \
    '        pending::apply_steps(&mut file.document, &already, steps);' \
    '        let _ = &already;
        pending::apply_steps(&mut file.document, &[], steps);'
expect_catch "io: a file is given a change it has already had" \
    "cargo test -p craftkit-io --profile ci a_file_is_given_only_the_changes_it_has_not_had"

# The rule itself: the list of changes a file still needs.
swap crates/io/src/format/pending.rs \
    '    names.iter().copied().filter(|name| !already.iter().any(|a| a == name)).collect()' \
    '    names.iter().copied().collect()'
expect_catch "io: the changes a file still needs are not worked out" \
    "cargo test -p craftkit-io --profile ci a_file_that_has_had_a_change_is_not_had_it_again"

# The control channel's methods, as the code has them and as the prose has
# them. §82: `AGENTS.md` said "Its methods are \u{2014}" and listed five of the
# eight, so a reader of the agent's own contract would not know that `ping`,
# `shutdown` or `diagnostics.crash_reports` existed.
swap AGENTS.md \
    '`diagnostics.list`, `diagnostics.crash_reports`, `ping`,' \
    '`diagnostics.list`,'
expect_catch "prose: a list of the control methods is missing one" \
    "cargo xtask lists"

# The chords the registry advertises, and the specification they are. §83:
# the tree binds no keys at all, and the field's own doc named "the UI's
# shortcut parser" — a parser that does not exist. The chords are the only
# thing the field is read for, so two commands on one chord is a
# specification nobody could implement.
swap crates/engine/src/commands.rs \
    '    shortcut: Some("M"),' \
    '    shortcut: Some("Ctrl+N"),'
expect_catch "engine: two commands advertise the same chord" \
    "cargo test -p craftkit-engine --profile ci the_advertised_shortcuts_are_unique_and_well_formed"

# The history's memory budget, on the undo path. §84: `push` enforced the
# budget and the undo path did not, so an undo that moved a bigger document
# into the redo stack left the two stacks holding more than the budget allows
# — measured at 67 936 429 bytes against a 67 108 864-byte budget.
swap crates/engine/src/history.rs \
    '        self.trim_to_budget();
        Some(entry.doc)
    }

    /// Take the state to restore, moving it to the undo stack.' \
    '        Some(entry.doc)
    }

    /// Take the state to restore, moving it to the undo stack.'
expect_catch "engine: an undo does not trim the history to its budget" \
    "cargo test -p craftkit-engine --profile ci undoing_does_not_take_the_history_over_its_budget"

# A property is named by a path, and a path is at most `MAX_PATH_SEGMENTS`
# segments long. The property tree was allowed to be deeper than that — it was
# bounded by `MAX_DEPTH`, which is the *node graph's* depth — and the listing
# stopped where the path stopped, so a deeper tree validated and then
# published nothing at all for its deepest property. Measured: a tree of
# seventeen groups validated, and `flatten` returned no properties for it.
# §85.
swap crates/model/src/property.rs \
    '        if depth >= limits::MAX_PATH_SEGMENTS {' \
    '        if depth > limits::MAX_DEPTH {'
expect_catch "model: the property tree may be deeper than a path can name" \
    "cargo test -p craftkit-model --profile ci a_tree_deeper_than_a_path_is_refused_rather_than_truncated"

# The listing counted name occurrences exactly; the resolver counted them
# case-insensitively. Two siblings called `Amount` and `amount` were both
# published as a first, so the second property's published path resolved to
# the *first* one, and the second had no path of its own. §85.
swap crates/model/src/property.rs \
    '    let key = label.to_ascii_lowercase();' \
    '    let key = label.clone();'
expect_catch "model: the listing counts names exactly while the resolver counts them loosely" \
    "cargo test -p craftkit-model --profile ci a_path_names_the_property_the_listing_published_it_for"

# The read side looked for an id *anywhere below* the group it was addressing;
# the write side looked only among that group's direct children. The engine
# writes, so `prop.set @5` — where property 5 sat one group down — answered
# "there is no such property" by inventing another one, and reported success.
# §85.
swap crates/model/src/property.rs \
    '        if let PathSeg::Uid(id) = last {
            return find_uid_in_mut(&mut group.nodes, *id);
        }' \
    '        if let PathSeg::Uid(id) = last {
            let _ = id;
        }'
expect_catch "model: the write side resolves an id differently from the read side" \
    "cargo test -p craftkit-engine --profile ci a_property_set_by_its_id_is_the_property_the_id_names"

# An id names a property that already exists; ids are the model's to assign.
# The engine used to read the segment as a literal name and create a property
# labelled after it — a property the grammar could not describe, so nothing
# could read it back. §85.
swap crates/model/src/property.rs \
    '            PathSeg::Uid(id) => {
                return Err(CoreError::BadPath(format!(
                    "`@{id:x}` names a property by its id, and there is no property with that id to create"
                )));
            }' \
    '            PathSeg::Uid(id) => {
                format!("{id:x}")
            }'
expect_catch "model: an id names a property that can be created" \
    "cargo test -p craftkit-engine --profile ci an_id_that_names_no_property_cannot_create_anything"

# A path may not be longer than the property tree may be deep. With the limit
# loosened, `prop.set` accepted a path of twenty-one segments, built the tree
# anyway, and was stopped by the validator afterwards — the command's answer
# came from the backstop, not from the grammar. §85.
swap crates/model/src/path.rs \
    '        if parts.is_empty() || parts.len() > limits::MAX_PATH_SEGMENTS {' \
    '        if parts.is_empty() || parts.len() > limits::MAX_PATH_SEGMENTS + 1 {'
expect_catch "model: a path may be longer than a property tree may be deep" \
    "cargo test -p craftkit-engine --profile ci a_path_too_long_to_name_is_refused"

# The token file outlives the process that wrote it, so the file from the last
# run is still there when the app starts. The live-window check used to wait
# for the file to *exist* and read that one — and the server refused it, so
# the check failed with a complaint about the control channel and a driver
# that had received nothing at all. §86.
swap scripts/live-window-check.sh \
    '            if [ "$now" != "$old" ]; then' \
    '            if [ -n "$now" ]; then'
expect_catch "live-window: a token left over from the last run is taken for this app's" \
    "scripts/live-window-check.sh --self-test"

# A name — a property's label or a group's id — becomes a path segment, so a
# name the grammar reads as something else is a name the grammar cannot
# write. The tree used to accept one: a file with the label `a/b` loaded, the
# listing published the path `a/b`, and `prop.set { path: "a/b" }` created a
# *second* property instead of changing the one on screen. §88.
swap crates/model/src/property.rs \
    '                    if let Some(why) = crate::path::why_not_writable(&p.label) {' \
    '                    if let Some(why) = None::<String>.filter(|_| false) {'
expect_catch "model: a property label the grammar cannot write is accepted anyway" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_a_name_the_grammar_cannot_write_is_refused"

# The same rule for a group's id.
swap crates/model/src/property.rs \
    '                    if let Some(why) = crate::path::why_not_writable(&g.id) {' \
    '                    if let Some(why) = None::<String>.filter(|_| false) {'
expect_catch "model: a group id the grammar cannot write is accepted anyway" \
    "cargo test -p craftkit-model --profile ci a_group_id_the_grammar_cannot_write_is_refused"

# The listing publishes a path, and the path has to be one the grammar can
# read. A name the grammar cannot write plainly is published with an explicit
# occurrence; without that, the published path names nothing at all. §88.
swap crates/model/src/path.rs \
    '                PathSeg::Match(name, n) => write!(f, "{}", segment_spelling(name, *n))?,' \
    '                PathSeg::Match(name, n) => write!(f, "{name}")?,'
expect_catch "model: a name is published as a spelling that does not parse back" \
    "cargo test -p craftkit-model --profile ci every_property_is_named_by_a_path_that_names_it"

# The frame rate's ceiling was written in five places, and the two that
# decided anything disagreed: the reader accepted a thousand frames per second
# and the command accepted two hundred and forty. A document could therefore
# hold a rate that no command could set -- not even the rate it already had.
# §89.
swap crates/engine/src/commands.rs \
    '                if f < 1 || f > i64::from(limits::MAX_FRAME_RATE) {' \
    '                if f < 1 || f > 240 {'
expect_catch "engine: the command's frame rate ceiling is not the model's" \
    "cargo test -p craftkit-engine --profile ci the_command_can_set_any_rate_a_document_may_carry"

# And the other way round: the model accepting a rate the command refuses.
swap crates/model/src/document.rs \
    '        if craftkit_core::FrameRate::new(self.meta.fps).is_err() || self.meta.fps > limits::MAX_FRAME_RATE {' \
    '        if craftkit_core::FrameRate::new(self.meta.fps).is_err() {'
expect_catch "model: a document may carry a frame rate the command refuses" \
    "cargo test -p craftkit-engine --profile ci the_command_can_set_any_rate_a_document_may_carry"

# Whether a rate divides the tick rate exactly is a fact the model can state.
# If it says "exact" for a rate that does not divide it, the tree promises
# exact frame boundaries it does not deliver -- and the error grows with the
# number of frames. §90.
swap crates/core/src/tick.rs \
    '        TICKS_PER_SECOND % self.0 as i128 == 0' \
    '        true'
expect_catch "core: a rate that does not divide the tick rate is called exact" \
    "cargo test -p craftkit-core --profile ci a_rate_that_does_not_divide_the_tick_rate_is_inexact"

# A keyframe list has rules — sorted, unique, within `MAX_KEYS`, on a time a
# reply could carry, with an easing the model accepts — and the model keeps
# them. A file is deserialized straight into the model, so the reader is the
# only place they can be enforced; before the check existed, a file carrying
# an unsorted list loaded, validated `Ok`, and animated to the *wrong value*.
# §91.
swap crates/model/src/property.rs \
    '                    p.keys.check(&p.label)?;' \
    '                    let _ = &p.keys;'
expect_catch "model: a property's keyframe list is not checked at all" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_broken_keyframes_is_refused"

# The list has to be sorted. An unsorted list does not sample to an untidy
# answer, it samples to the wrong one: the key that is earliest in time is
# never reached. §91.
swap crates/model/src/keyframe.rs \
    '                if key.time < prev.time {' \
    '                if key.time < prev.time && false {'
expect_catch "model: a keyframe list that is not sorted is accepted" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_broken_keyframes_is_refused"

# Two keys at the same time: of the two, only the second was ever reachable.
swap crates/model/src/keyframe.rs \
    '                if key.time == prev.time {' \
    '                if key.time == prev.time && false {'
expect_catch "model: two keyframes at the same time are accepted" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_broken_keyframes_is_refused"

# A time outside the range a reply could carry. The tick's own documentation
# promises that the reader refuses such a file. §60, §91.
swap crates/model/src/keyframe.rs \
    '            if raw < Tick::MIN.raw() || raw > Tick::MAX.raw() {' \
    '            if (raw < Tick::MIN.raw() || raw > Tick::MAX.raw()) && false {'
expect_catch "model: a keyframe on a time no reply could carry is accepted" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_broken_keyframes_is_refused"

# The easing the model's own constructor refuses must not arrive by another
# door. §91.
swap crates/model/src/keyframe.rs \
    '            key.ease
                .checked()
                .map_err(|e| CoreError::BadData(format!("the easing of a keyframe of {what}: {e}")))?;' \
    '            let _ = key.ease.checked();'
expect_catch "model: an easing the model refuses is accepted" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_broken_keyframes_is_refused"

# And the easing's own check: the control point's range.
swap crates/model/src/keyframe.rs \
    '            craftkit_core::in_range("easing out_x", out_x as f64, 0.0, 1.0)?;' \
    '            let _ = out_x as f64;'
expect_catch "model: a bezier control point outside 0..=1 is accepted" \
    "cargo test -p craftkit-model --profile ci a_non_finite_control_point_is_refused"

# The limit.
swap crates/model/src/keyframe.rs \
    '        if self.0.len() > crate::limits::MAX_KEYS {' \
    '        if self.0.len() > crate::limits::MAX_KEYS && false {'
expect_catch "model: a property may carry more keyframes than the limit" \
    "cargo test -p craftkit-model --profile ci more_keys_than_the_limit_are_refused"

# And the door the easing comes through when a command changes it.
swap crates/model/src/keyframe.rs \
    '        let ease = ease.checked()?;' \
    '        let ease = ease;'
expect_catch "model: set_ease stores an easing the model refuses" \
    "cargo test -p craftkit-model --profile ci set_ease_changes_the_curve_and_refuses_an_invalid_one"

# A name is not empty and not blank. The tree had three answers to "what is a
# name?": the command accepted an empty name and the validator refused it
# afterwards, so the user's empty name came back as the tree's own unexpected
# failure -- the kind the tree documents as "worth a crash report and a bug".
# And `node.add { name: " " }` was accepted while `node.rename` refused the
# same name. §92.
swap crates/model/src/document.rs \
    '        if name.trim().is_empty() {' \
    '        if false {'
expect_catch "model: an empty or blank name is a name" \
    "cargo test -p craftkit-engine --profile ci an_empty_node_name_is_a_refusal_not_an_internal_failure"

# The length is counted in bytes, like every other text limit in the tree. The
# engine had a copy of the rule counted in characters, which could never fire:
# the parameter had already been refused by the byte count. §92.
swap crates/model/src/document.rs \
    '        if name.len() > limits::MAX_NAME {' \
    '        if name.chars().count() > limits::MAX_NAME {'
expect_catch "model: a name's length is counted in characters" \
    "cargo test -p craftkit-model --profile ci the_name_rule_says_what_a_name_may_not_be"

# The constructor keeps the rule the validator keeps. Otherwise a command can
# add a node the document cannot hold, and the validator is what notices --
# after the damage. §92.
swap crates/model/src/document.rs \
    '        if let Some(why) = Node::why_not_a_name(&name) {' \
    '        if let Some(why) = None::<String> {'
expect_catch "model: the constructor accepts a name the validator refuses" \
    "cargo test -p craftkit-model --profile ci the_constructor_refuses_a_name_the_validator_would_refuse"

# And the validator keeps it for the files, which are deserialized straight
# into the model. §92.
swap crates/model/src/document.rs \
    '            if let Some(why) = Node::why_not_a_name(&n.name) {' \
    '            if let Some(why) = None::<String> {'
expect_catch "model: a file may carry a node whose name cannot be used" \
    "cargo test -p craftkit-io --profile ci a_file_carrying_an_unusable_node_name_is_refused"

# The command asks the model for the rule, so the two cannot disagree. §92.
swap crates/engine/src/commands.rs \
    '    if let Some(why) = craftkit_model::Node::why_not_a_name(name) {' \
    '    if let Some(why) = None::<String> {'
expect_catch "engine: the command does not ask the model what a name is" \
    "cargo test -p craftkit-engine --profile ci an_empty_node_name_is_a_refusal_not_an_internal_failure"

# Asking a command to change nothing is the user's mistake, not the tree's.
# It used to reach the backstop -- "the command declared that it mutates the
# document but changed nothing" -- which is an internal error. §92.
swap crates/engine/src/commands.rs \
    '        if !p.has("start") && !p.has("end") && !p.has("fps") {' \
    '        if false {'
expect_catch "engine: a command asked to change nothing is reported as a crash" \
    "cargo test -p craftkit-engine --profile ci a_range_command_with_nothing_to_change_is_a_refusal"

# A parameter that was given but is empty is not "required". §92.
swap crates/engine/src/commands.rs \
    '            source: craftkit_core::ParamError::new(key, "this parameter must not be empty"),' \
    '            source: craftkit_core::ParamError::new(key, "this parameter is required"),'
expect_catch "engine: a parameter that was given but is empty says it is required" \
    "cargo test -p craftkit-engine --profile ci a_parameter_that_was_given_but_is_empty_says_so"

# And every command that takes a name agrees about what a name is. The two
# mutations above are what break the agreement -- the model's rule, and the
# command asking it -- so there is no separate case for it: a mutation that
# cannot be seen is not a case. §92.

# The test that promises to fire every command at every documented parameter
# name derives its values from the registry. It used to fire five fixed
# objects, and 27 documented parameters were never fired at all. §92.
swap crates/engine/src/session.rs \
    '        let per_parameter: Vec<(String, Vec<serde_json::Value>)> = command::ids()' \
    '        let per_parameter: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
        let _unused: Vec<(String, Vec<serde_json::Value>)> = command::ids()'
expect_catch "engine: a documented parameter is never fired with a hostile value" \
    "cargo test -p craftkit-engine --profile ci every_documented_parameter_is_fired_with_a_hostile_value"

# A command asked to do what it has already done is a success, not a crash.
# The engine compares the document with the snapshot it took before the
# command, and cannot tell "the command forgot to write" from "the command
# wrote the value the field already held" -- so the command says, and the
# engine believes it. Measured: eight of the nine commands that can be asked
# to do what they have already done were refused as "failed unexpectedly
# (the command declared that it mutates the document but changed nothing)".
# §93.
swap crates/engine/src/session.rs \
    '                    if !changed && outcome.changed {' \
    '                    if !changed {'
expect_catch "engine: a command asked to do what it has already done is a crash" \
    "cargo test -p craftkit-engine --profile ci a_command_asked_to_do_what_it_has_already_done_succeeds"

# And the other way round: a command that claims to have changed something and
# did not is still a bug -- that is what §44's check is for, and the default
# is the direction that keeps it alive. §93.
swap crates/engine/src/session.rs \
    '                    if !changed && outcome.changed {' \
    '                    if false {'
expect_catch "engine: a command that changed nothing is not caught" \
    "cargo test -p craftkit-engine --profile ci a_command_that_claims_to_change_something_and_does_not_is_still_refused"

# A no-op must not put a step on the undo stack, nor mark the project dirty.
swap crates/engine/src/session.rs \
    '                    if !changed {
                        // The command says it changed nothing, and the' \
    '                    if false {
                        // The command says it changed nothing, and the'
expect_catch "engine: a no-op puts a step on the undo stack" \
    "cargo test -p craftkit-engine --profile ci a_command_asked_to_do_what_it_has_already_done_succeeds"

# The commands that may do nothing have to say so. Without it, the engine
# sees a document it did not expect and refuses the command. §93.
swap crates/engine/src/commands.rs \
    '        let changed = node_mut(s, id, NODE_MUTE.id)?.set_muted(muted);' \
    '        node_mut(s, id, NODE_MUTE.id)?.set_muted(muted);
        let changed = true;'
expect_catch "engine: muting a node that is already muted is a crash" \
    "cargo test -p craftkit-engine --profile ci a_command_asked_to_do_what_it_has_already_done_succeeds"

swap crates/engine/src/commands.rs \
    '                let mut changed = existing.set_value(value.clone());' \
    '                existing.set_value(value.clone());
                let mut changed = true;'
expect_catch "engine: setting a property to the value it already has is a crash" \
    "cargo test -p craftkit-engine --profile ci a_command_asked_to_do_what_it_has_already_done_succeeds"

# The write that says whether it changed anything. Without it, every command
# that uses it has to guess. §93.
swap crates/model/src/property.rs \
    '    pub fn set_value(&mut self, value: Value) -> bool {
        if self.value == value {
            return false;
        }' \
    '    pub fn set_value(&mut self, value: Value) -> bool {
        if false {
            return false;
        }'
expect_catch "model: a write that changes nothing does not say so" \
    "cargo test -p craftkit-model --profile ci writing_a_value_a_property_already_holds_says_it_changed_nothing"

# A command that does not change the document has nothing to undo, so it
# cannot be undoable: the push lives inside the `mutates` branch. Four
# commands -- `playback.seek`, `plan.evaluate`, `diagnostics.list` and
# `session.stats` -- declared `undoable: true` and never could, and the flag
# is published to agents in the command schema. §94.
#
# The anchor names one command. It used to be the pair of flags alone, and five
# commands have that pair, so `swap` replaced the first of them and the case
# was a case about whichever command happened to be first. The rule it breaks
# -- a command that does not mutate is not undoable -- is global over the
# commands, so it caught the break; but an anchor that names five places names
# none of them, and the anchors gate says so. §184.
#
# The comment above used to say that a longer anchor "would run into the
# apostrophe in the plan's rate, and an apostrophe closes the shell's
# single-quoted string". That is not true. The shell writes an apostrophe
# inside a single-quoted string as '"'"', and this script does it in twenty-one
# places. The anchor below has no apostrophe in it; it could have had one.
swap crates/engine/src/commands.rs \
    '    mutates: false,
    undoable: false,
    enabled: has_nodes,' \
    '    mutates: false,
    undoable: true,
    enabled: has_nodes,'
expect_catch "engine: a command that does not mutate is undoable anyway" \
    "cargo test -p craftkit-engine --profile ci no_command_that_does_not_mutate_is_undoable"

# The other direction of the pending list: the changes a file HAS HAD that this
# build does not have. §95: the reader worked out what a file still needed and
# never asked what it had already had, so a file written by a build with a
# format change this build does not have was read like any other -- and the
# parts of the document that change had put there were dropped without a word.
swap crates/io/src/format/pending.rs \
    '.filter(|name| !known.contains(&name.as_str()))' \
    '.filter(|_| false)'
expect_catch "io: the changes a file has that this build does not have are not worked out" \
    "cargo test -p craftkit-io --profile ci the_changes_a_file_has_that_this_build_does_not_have_are_worked_out"

# The refusal itself. A pending change does not bump the version, so the tag
# cannot catch a file from a build with a change this build does not have: the
# file says craftkit/1, like every file this build writes. The reader has to
# refuse it on the strength of the list alone.
swap crates/io/src/format.rs \
    '    if version != current {' \
    '    if version == current {'
expect_catch "io: a file naming a change this build has never heard of is read" \
    "cargo test -p craftkit-io --profile ci a_file_that_names_a_change_this_build_has_never_heard_of_is_refused"

# The known set has to include the steps the seam hands in, or the refusal
# above would refuse the test that section 81 needs: a file whose header names
# a change this reader DOES have.
swap crates/io/src/format.rs \
    '        for (name, _) in steps {
            if !known.contains(name) {
                known.push(name);
            }
        }' \
    '        for (name, _) in steps {
            let _ = name;
        }'
expect_catch "io: the reader forgets the changes the seam hands it" \
    "cargo test -p craftkit-io --profile ci a_file_is_given_only_the_changes_it_has_not_had"

# The refusal has to name the change, or the person holding the file cannot
# tell which one it is.
swap crates/io/src/format.rs \
    '            Some(name) => Err(FormatError::UnknownChange(name.clone())),' \
    '            Some(_name) => Err(FormatError::UnknownChange(String::new())),'
expect_catch "io: the refusal does not name the change" \
    "cargo test -p craftkit-io --profile ci a_file_that_names_a_change_this_build_has_never_heard_of_is_refused"

# The other side of the `mutates` flag. A command that says it does not change
# the document and changes it is a bug, and a quiet one: the change would not
# be validated, would not be undoable, and would not mark the project dirty.
# Every backstop the engine has lived inside the `mutates` branch, so nothing
# was on this side. §96.
swap crates/engine/src/session.rs \
    '                } else if self.doc != snapshot {' \
    '                } else if self.doc == snapshot {'
expect_catch "engine: a command that writes while claiming to be read-only is not caught" \
    "cargo test -p craftkit-engine --profile ci a_command_that_declares_it_does_not_mutate_and_writes_is_refused"

# The refusal has to put the document back. Without the restore the write
# stays, and the session holds a design the file reader would refuse.
#
# The anchor runs over two lines on purpose: the restore line on its own also
# matches the deeper ones in the `mutates` branch, and `swap` replaces the
# first match, which is one of those.
swap crates/engine/src/session.rs \
    '                    self.restore(snapshot, before_dirty);
                    let err = EngineError::Internal {
                        id: id.to_string(),
                        detail: "the command declared that it does not mutate the document but changed it"' \
    '                    let _ = before_dirty;
                    let err = EngineError::Internal {
                        id: id.to_string(),
                        detail: "the command declared that it does not mutate the document but changed it"'
expect_catch "engine: a command that writes while claiming to be read-only is not rolled back" \
    "cargo test -p craftkit-engine --profile ci a_command_that_declares_it_does_not_mutate_cannot_leave_an_invalid_document"

# The refusal has to say what the command did, or the person reading the log
# has to guess which of the two backstops it was.
swap crates/engine/src/session.rs \
    '                        detail: "the command declared that it does not mutate the document but changed it"' \
    '                        detail: "the command changed the document"'
expect_catch "engine: the refusal does not say what the command did" \
    "cargo test -p craftkit-engine --profile ci a_command_that_declares_it_does_not_mutate_and_writes_is_refused"

# A group's id is a name, and every name in the model is at most 255 bytes.
# The group id had no rule at all, so a file could carry a group whose id was
# thirty thousand bytes: the document was read, it validated, and the listing
# published a path no command would carry. §97.
swap crates/model/src/property.rs \
    '                    if g.id.is_empty() || g.id.len() > limits::MAX_NAME {' \
    '                    if false {'
expect_catch "model: a group id longer than a name is accepted" \
    "cargo test -p craftkit-model --profile ci a_group_id_longer_than_a_name_is_refused"

# The grammar refused a name of 256 bytes and accepted the same name written
# with an explicit occurrence, because the length rule was only in the plain
# branch. The listing used that spelling for a long group id.
swap crates/model/src/path.rs \
    '            if name.len() > limits::MAX_NAME {' \
    '            if false {'
expect_catch "model: a name too long is accepted in the occurrence spelling" \
    "cargo test -p craftkit-model --profile ci a_name_too_long_is_refused_in_either_spelling"

# A path is made of names, so a path may be longer than a name. The commands
# capped a path at a name's length, so the tree published paths it refused.
swap crates/engine/src/commands.rs \
    '    required_text_capped(p, key, limits::MAX_PATH_BYTES)' \
    '    required_text_capped(p, key, limits::MAX_NAME)'
expect_catch "engine: a path the grammar can express is not carried" \
    "cargo test -p craftkit-engine --profile ci a_path_longer_than_a_name_is_still_carried"

# The reader validates the document it read. Without that, a file whose group
# id is longer than a name is read: the model's rule is there, but nothing
# asks the model. §97.
swap crates/io/src/format.rs \
    '        file.document.validate().map_err(|e| FormatError::InvalidDocument(e.to_string()))?;' \
    '        let _ = &file.document;'
expect_catch "io: a file with a group id longer than a name is read" \
    "cargo test -p craftkit-io --profile ci a_file_with_a_group_id_longer_than_a_name_is_refused"

# The automation registry checked for a duplicate name by walking the list of
# the names it had already published -- once per widget, on every frame. A
# document at the model's own limit of ten thousand nodes publishes twenty
# thousand names, and the frame spent 378 ms in the registry. §98.
swap crates/ui/src/automation.rs \
    '        if self.seen.insert(id.clone()) {
            self.entries.push((id, rect, enabled));
        }' \
    '        if !self.entries.iter().any(|(known, _, _)| known == &id) {
            self.entries.push((id, rect, enabled));
        }'
expect_catch "ui: the registry scans every name it has published for every widget" \
    "cargo test -p craftkit-ui --profile ci publishing_a_widget_does_not_cost_more_than_the_widgets_before_it"

# ------------------------------------------------------------------ done ----
# The level bar wore a property's name. The node list published the level bar
# as `node.<id>.prop.$level` -- which is how a property *named* `$level`
# spells its own name -- so the level bar and the property's editor wanted
# one name. The first publication won and the second was not in the registry
# at all, so an agent asking for the property's editor was answered with the
# level bar's rectangle, in another panel. §99.
swap crates/ui/src/automation.rs \
    '    pub fn level_bar(id: craftkit_core::NodeId) -> Self {
        AutomationId(format!("node.{}.level", id.get()))
    }' \
    '    pub fn level_bar(id: craftkit_core::NodeId) -> Self {
        AutomationId(format!("node.{}.prop.$level", id.get()))
    }'
expect_catch "ui: the level bar wears a property's name" \
    "cargo test -p craftkit-ui --profile ci a_level_bar_and_a_property_never_share_a_name"

# The level bar published its *fill* -- the coloured part, whose width is the
# level -- so at level zero the name resolved to a rectangle with no width at
# all: a widget an agent could look up and never click. §99.
swap crates/ui/src/shell.rs \
    '                    let (level_track, _) = draw_level_bar(ui, tokens, row.level);' \
    '                    let (_, level_track) = draw_level_bar(ui, tokens, row.level);'
expect_catch "ui: the level bar publishes the fill" \
    "cargo test -p craftkit-ui --profile ci a_level_bar_publishes_a_rectangle_with_an_area"

# A name two widgets want was swallowed: the first was published and the
# second was not in the registry at all, so nothing said that the widget
# behind the name was not the one that was asked for. §99.
swap crates/ui/src/automation.rs \
    '        } else {
            self.collisions.push(id);
        }' \
    '        } else {
            let _ = id;
        }'
expect_catch "ui: a name two widgets want is not recorded" \
    "cargo test -p craftkit-ui --profile ci a_name_two_widgets_want_is_recorded_as_a_collision"

# The registry was never cleared, so it grew for as long as the shell lived
# and "the first publication wins" meant "the first *frame* wins": a widget
# that had gone kept its rectangle, and an agent's click would land on
# nothing. §99.
swap crates/ui/src/shell.rs \
    '        self.automation.clear();' \
    '        let _ = &self.automation;'
expect_catch "ui: the registry is not rebuilt each frame" \
    "cargo test -p craftkit-ui --profile ci the_registry_holds_the_frame_that_was_drawn"

# The node list drew a mute and a delete button in every row and published
# neither, so the registry did not know they were there: "what is at this
# point?" answered *nothing* at the pixels where a button was. §100.
swap crates/ui/src/shell.rs \
    '                    let mute = ui.small_button("mute");
                    automation.publish(AutomationId::node_mute(id), mute.rect, true);' \
    '                    let mute = ui.small_button("mute");'
expect_catch "ui: the row's buttons are not published" \
    "cargo test -p craftkit-ui --profile ci every_node_row_has_its_buttons_named"

# The "clear keys" button was published as `node.<id>.key.<path>` -- the name
# of a keyframe row, a widget the shell does not draw. A name that promises a
# widget that is not on screen sends an agent looking for something it will
# never find. §100.
swap crates/ui/src/automation.rs \
    '        AutomationId(format!("node.{}.keys.{}.clear", id.get(), path))' \
    '        AutomationId(format!("node.{}.key.{}", id.get(), path))'
expect_catch "ui: the clear-keys button promises a row that is not drawn" \
    "cargo test -p craftkit-ui --profile ci a_name_never_promises_a_widget_that_is_not_drawn"

# The playhead slider -- the timeline's one control -- was published nowhere.
# §100.
swap crates/ui/src/shell.rs \
    '        self.automation.publish(AutomationId::playhead(), response.rect, true);' \
    '        let _ = &response;'
expect_catch "ui: the playhead is not published" \
    "cargo test -p craftkit-ui --profile ci the_timeline_has_a_name"

# The "key at the playhead" path field was published nowhere, so a field an
# agent could not name was a field it could not fill. §100.
swap crates/ui/src/shell.rs \
    '        automation.publish(AutomationId::key_path(), path_field.rect, true);' \
    '        let _ = &path_field;'
expect_catch "ui: the key path field is not published" \
    "cargo test -p craftkit-ui --profile ci the_key_fields_have_names"

# A widget published with another widget's rectangle: the registry answers for
# the wrong thing at that point, and the name it publishes is a name that does
# not lead to it. §100.
swap crates/ui/src/shell.rs \
    '                    let (level_track, _) = draw_level_bar(ui, tokens, row.level);
                    automation.publish(AutomationId::node_row(id), row_rect, true);
                    automation.publish(AutomationId::level_bar(id), level_track, true);' \
    '                    let (_level_track, _) = draw_level_bar(ui, tokens, row.level);
                    automation.publish(AutomationId::node_row(id), row_rect, true);
                    automation.publish(AutomationId::level_bar(id), row_rect, true);'
expect_catch "ui: a widget is published with another widget's rectangle" \
    "cargo test -p craftkit-ui --profile ci every_published_name_answers_at_its_own_centre"

# The control channel published the *names* of the widgets and not their
# positions, and the click took pixels -- so an agent that knew a widget's
# name could not act on it and had to guess where the name was. §104.
swap crates/ui/src/raster.rs \
    '    let centre = rect.center();' \
    '    let centre = egui::pos2(width as f32 / 2.0, height as f32 / 2.0);'
expect_catch "ui: a click by name lands somewhere else" \
    "cargo test -p craftkit-ui --profile ci a_click_by_name_lands_on_that_widget"

# The widgets reply carries the rectangles, so an agent can say where a
# widget is without guessing. §104.
swap apps/craftkit/src/main.rs \
    '                        Json::from(rect.width()),' \
    '                        Json::from(0.0),'
expect_catch "ui: the published rectangles have no width" \
    "cargo test -p craftkit --profile ci the_widgets_reply_says_where_the_widgets_are"

# The desktop app built a fresh shell every frame and threw the shell's state
# away with it, so the selection was gone before the next frame and the
# properties panel could never open. The frame now *takes* the state as well
# as giving it back. §105.
swap crates/ui/src/shell.rs \
    '    shell.state = state.clone();' \
    '    shell.state = ShellState::default();'
expect_catch "ui: the frame forgets the state it was given" \
    "cargo test -p craftkit-ui --profile ci a_selection_made_in_one_frame_is_there_in_the_next"

# ...and the other way round: the frame draws with the host's state but never
# gives back what the frame changed, so a selection made during the frame was
# lost just the same. §105.
swap crates/ui/src/shell.rs \
    '    *state = shell.state.clone();' \
    '    let _ = &shell.state;'
expect_catch "ui: the frame does not give the state back" \
    "cargo test -p craftkit-ui --profile ci a_selection_made_in_one_frame_is_there_in_the_next"

# The "key at the playhead" path field kept its text in a local, which is
# rebuilt every frame: every keystroke was thrown away. §105.
swap crates/ui/src/shell.rs \
    '        let path_field =
            ui.add(egui::TextEdit::singleline(&mut state.key_path).hint_text("path, e.g. amount"));' \
    '        let mut path_text = String::new();
        let path_field =
            ui.add(egui::TextEdit::singleline(&mut path_text).hint_text("path, e.g. amount"));'
expect_catch "ui: the key path field keeps its text in a local" \
    "cargo test -p craftkit-ui --profile ci a_path_typed_into_the_key_field_is_still_there_in_the_next_frame"

# The value field appeared only on the frame the path changed, so it was gone
# before anything could be typed into it. §105.
swap crates/ui/src/shell.rs \
    '        if !state.key_path.is_empty() {' \
    '        if path_field.changed() {'
expect_catch "ui: the key value field is only there on the frame the path changed" \
    "cargo test -p craftkit-ui --profile ci a_path_typed_into_the_key_field_is_still_there_in_the_next_frame"

# The render drew the shell with nothing selected, so the properties panel was
# empty -- and every property editor had no name to be found under. §106.
swap crates/ui/src/raster.rs \
    '    let mut shell = crate::shell::Shell::with_state(session, state.clone());
    let mut deltas: Vec<egui::TexturesDelta> = Vec::new();' \
    '    let mut shell = crate::shell::Shell::new(session);
    let mut deltas: Vec<egui::TexturesDelta> = Vec::new();'
expect_catch "ui: the render draws the shell with nothing selected" \
    "cargo test -p craftkit-ui --profile ci the_state_decides_what_is_on_the_screen"

# The click by name settled the layout with nothing selected, so a property
# editor was not on the screen to be found. §106.
swap crates/ui/src/raster.rs \
    '    let mut shell = crate::shell::Shell::with_state(session, state.clone());
    for _ in 0..FRAMES {
        let _ = ctx.run(raw.clone(), |ctx| shell.show(ctx, &tokens));
    }

    let rect = widget_rect(&shell.automation, name)?;' \
    '    let mut shell = crate::shell::Shell::new(session);
    for _ in 0..FRAMES {
        let _ = ctx.run(raw.clone(), |ctx| shell.show(ctx, &tokens));
    }

    let rect = widget_rect(&shell.automation, name)?;'
expect_catch "ui: a click by name looks for the widget on the empty screen" \
    "cargo test -p craftkit-ui --profile ci a_property_editor_can_be_clicked_by_name_when_a_node_is_selected"

# The channel was handed a state of its own instead of the window's, so the
# screen it described was not the screen the window was showing. §106.
swap apps/craftkit/src/main.rs \
    '            let guard = ui_state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            ui_request(session, request, &guard)' \
    '            let _guard = ui_state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            ui_request(session, request, &craftkit_ui::shell::ShellState::default())'
expect_catch "ui: the channel reports a screen of its own" \
    "cargo test -p craftkit --profile ci the_channel_reports_the_windows_screen_over_the_socket"

# The `selected` parameter was ignored, so an agent could not ask about a
# screen with a node selected -- and the properties panel stayed invisible. §106.
swap apps/craftkit/src/main.rs \
    '    state.selected = Some(node);' \
    '    state.selected = None;'
expect_catch "ui: the selected node is ignored" \
    "cargo test -p craftkit --profile ci the_property_editors_are_on_the_screen_when_a_node_is_selected"

# A node that is not in the document was accepted, and answered with an empty
# panel instead of a refusal. §106.
swap apps/craftkit/src/main.rs \
    '    if session.node(node).is_none() {' \
    '    if session.node(node).is_some() {'
expect_catch "ui: a selected node that is not there is accepted" \
    "cargo test -p craftkit --profile ci a_selected_node_that_is_not_there_is_refused"

# The click by point answered "that button is at that point" for a button
# that was greyed out -- a widget that cannot be pressed. The click by name
# said so properly, so one question had two answers. §107.
swap crates/ui/src/raster.rs \
    '    if let Some(found) = hit
        && !found.enabled
    {
        return Err(format!("`{}` is on screen but disabled, so there is nothing to click", found.name));
    }' \
    '    if let Some(found) = hit
        && !found.enabled
        && false
    {
        return Err(format!("`{}` is on screen but disabled, so there is nothing to click", found.name));
    }'
expect_catch "ui: a click on a disabled widget is not refused" \
    "cargo test -p craftkit-ui --profile ci a_click_on_a_disabled_widget_is_refused"

# The registry answered with the name and dropped the flag it already held,
# so the answer at a point could not say whether the widget can be clicked.
# §107.
swap crates/ui/src/automation.rs \
    '            .map(|(id, _, enabled)| Hit { name: id.0.as_str(), enabled: *enabled })' \
    '            .map(|(id, _, _)| Hit { name: id.0.as_str(), enabled: true })'
expect_catch "ui: the answer at a point does not say whether it can be clicked" \
    "cargo test -p craftkit-ui --profile ci the_answer_at_a_point_says_whether_it_can_be_clicked"

# The list of the widgets that cannot be clicked was the list of the ones
# that can -- so a greyed-out button looked exactly like a live one. §107.
swap crates/ui/src/automation.rs \
    '            self.entries.iter().filter(|(_, _, enabled)| !*enabled).map(|(id, _, _)| id.0.as_str()).collect();' \
    '            self.entries.iter().filter(|(_, _, enabled)| *enabled).map(|(id, _, _)| id.0.as_str()).collect();'
expect_catch "ui: the dead list is the list of the live ones" \
    "cargo test -p craftkit --profile ci the_widgets_reply_says_which_widgets_cannot_be_clicked"

# A widget that is not there at all is refused as though it were on the
# screen but disabled -- so the two refusals said the same thing about two
# different situations. §104's rule, and §107's. §107.
swap crates/ui/src/raster.rs \
    '        crate::automation::Widget::Missing => Err(format!("there is no widget named `{name}` on screen")),' \
    '        crate::automation::Widget::Missing => Err(format!("`{name}` is on screen but disabled, so there is nothing to click")),'
expect_catch "ui: a widget that is not there is refused as though it were disabled" \
    "cargo test -p craftkit-ui --profile ci a_click_by_a_name_that_is_not_there_is_refused"

# The frame rate could only be *set*, never read: the state had the document's
# name but not its rate, so the only way to ask for it was to set it, which
# rewrote the design. §108.
swap crates/engine/src/session.rs \
    '            "fps": self.doc.meta.fps,' \
    '            "fps": 0,'
expect_catch "engine: the state does not carry the frame rate" \
    "cargo test -p craftkit-engine --profile ci the_state_and_the_range_command_agree_about_the_rate"

# The range is the document's own, and the state is the only place an agent
# can read it. §108.
swap crates/engine/src/session.rs \
    '            "start": self.doc.meta.start.raw(),' \
    '            "start": self.doc.meta.end.raw(),'
expect_catch "engine: the state does not carry the document's start" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_document_s_own_parameters"

# ...and the end of it, which is the other half of the range. §108.
swap crates/engine/src/session.rs \
    '            "end": self.doc.meta.end.raw(),' \
    '            "end": 0,'
expect_catch "engine: the state does not carry the document's end" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_document_s_own_parameters"

# The session kept the last 256 commands and could only be asked how many had
# run. The commands were in the tree the whole time. §109.
swap crates/engine/src/session.rs \
    '            "log": self.log.entries(),' \
    '            "log": [],'
expect_catch "engine: the state does not carry the commands that ran" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_commands_that_ran"

# ...and the log carried the last command only, so "what has this session
# done?" had one answer. §109.
swap crates/engine/src/session.rs \
    '            "log": self.log.entries(),' \
    '            "log": self.log.tail(1),'
expect_catch "engine: the log carries only the last command" \
    "cargo test -p craftkit-engine --profile ci the_log_and_the_command_count_agree"

# The log said which commands ran but not whether they worked, and a refusal
# is the one an agent most wants to see. §109.
swap crates/engine/src/log.rs \
    '    pub ok: bool,' \
    '    #[serde(skip)]
    pub ok: bool,'
expect_catch "engine: the log does not say whether the command worked" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_commands_that_ran"

# The note had no bound at all, so it could be any size -- and the state
# carries it whole, so an unbounded note is an unbounded reply. §110.
swap crates/model/src/document.rs \
    '        if self.meta.comment.len() > limits::MAX_COMMENT_BYTES {' \
    '        if self.meta.comment.len() > limits::MAX_COMMENT_BYTES + 1 {'
expect_catch "model: a note one byte over the limit is accepted" \
    "cargo test -p craftkit-model --profile ci a_note_over_the_byte_limit_is_refused"

# The setter said it changed the note even when the note was the note that was
# already there, so the engine's "you changed nothing" backstop fired on an
# ordinary command. §110.
swap crates/model/src/document.rs \
    '        if self.meta.comment == comment {
            return false;
        }' \
    '        if self.meta.comment == comment {
            return true;
        }'
expect_catch "model: the note setter always says it changed something" \
    "cargo test -p craftkit-engine --profile ci writing_the_same_note_changes_nothing"

# The state is the only place an agent can read the design's note, and it did
# not carry it. §110.
swap crates/engine/src/session.rs \
    '            "comment": self.doc.meta.comment,' \
    '            "comment": "",'
expect_catch "engine: the state does not carry the note" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_note"

# The command wrote the note without a bound, so the note the file refuses was
# a note the command wrote. §110.
swap crates/engine/src/commands.rs \
    '            .text("text", limits::MAX_COMMENT_BYTES)' \
    '            .text("text", limits::MAX_COMMENT_BYTES + 1)'
expect_catch "engine: the note command accepts a note over the limit" \
    "cargo test -p craftkit-engine --profile ci a_note_over_the_limit_is_refused"

# The command claimed it had changed the note when it had not, so the engine
# refused an ordinary command as though it had forgotten to write. §110.
swap crates/engine/src/commands.rs \
    '        let changed = s.doc_mut().set_comment(text.clone());' \
    '        s.doc_mut().set_comment(text.clone());
        let changed = true;'
expect_catch "engine: the note command always claims it changed something" \
    "cargo test -p craftkit-engine --profile ci writing_the_same_note_changes_nothing"

# The note was computed and never drawn, so the window showed a design without
# its note. §110.
swap crates/ui/src/shell.rs \
    '        if !meta.comment.is_empty() {' \
    '        if !meta.comment.is_empty() && false {'
expect_catch "ui: the note is not drawn" \
    "cargo test -p craftkit-ui --profile ci a_frame_with_a_note_is_not_the_frame_without_one"

# The preview did not cut, so a long note was drawn in full over the panel --
# and a short one was marked as cut when it was not. §110.
swap crates/ui/src/shell.rs \
    '    if note.chars().count() <= NOTE_PREVIEW_CHARS {' \
    '    if note.chars().count() <= NOTE_PREVIEW_CHARS && false {'
expect_catch "ui: the note preview does not say that it was cut" \
    "cargo test -p craftkit-ui --profile ci the_note_preview_is_a_glimpse_of_the_note"

# `eval` printed the node count and the frame rate and never the note, so the
# only way to read a design's note was to open the file. §110.
swap apps/craftkit-cli/src/cli.rs \
    '    if note.is_empty() { None } else { Some(format!("note: {note}")) }' \
    '    if note.is_empty() || true { None } else { Some(format!("note: {note}")) }'
expect_catch "cli: eval does not say what the design's note is" \
    "cargo test -p craftkit-cli --profile ci eval_says_what_the_design_s_note_is"

# The state carried two booleans about the undo stack, and the depth was in
# the statistics -- while the state's own doc comment said it was in the
# state. §111.
swap crates/engine/src/session.rs \
    '            "undo_depth": self.undo_depth(),' \
    '            "undo_depth": 0,'
expect_catch "engine: the state does not carry the undo depth" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_undo_stack"

# ...and the undo stack's memory was in nothing at all, although the tree
# bounds it and measures it on every push. §111.
swap crates/engine/src/session.rs \
    '            "undo_bytes": self.history.bytes(),' \
    '            "undo_bytes": 0,'
expect_catch "engine: the state does not carry the undo stack's memory" \
    "cargo test -p craftkit-engine --profile ci the_state_carries_the_undo_stack"

# Two objects, one stack: the statistics and the state may not disagree about
# how deep it is. §111.
swap crates/engine/src/commands.rs \
    '                "undo_depth": s.undo_depth(),' \
    '                "undo_depth": s.undo_depth() + 1,'
expect_catch "engine: the statistics and the state disagree about the undo depth" \
    "cargo test -p craftkit-engine --profile ci the_state_and_the_statistics_agree_about_the_undo_stack"

# The browser panel had a flag for "there is a plan saved here", wrote it three
# times and read it never, and the line that would have used it could never be
# shown. §112.
swap apps/craftkit-web/src/main.rs \
    '        text: if storage.saved {
            "This browser holds one plan, and it is saved.".to_string()
        } else {
            "This browser holds one plan slot, and nothing is saved in it yet.".to_string()
        },' \
    '        text: "This browser holds one plan.".to_string(),'
expect_catch "ui: the browser panel does not say whether a plan is saved" \
    "cargo test -p craftkit-web --profile ci the_panel_says_whether_a_plan_is_saved"

# The panel coloured its note by the shell's error flag, so a save the browser
# refused was drawn in the ordinary colour. §112.
swap apps/craftkit-web/src/main.rs \
    '        lines.push(PanelLine { text: storage.note.clone(), warning: storage.failed });' \
    '        lines.push(PanelLine { text: storage.note.clone(), warning: false });'
expect_catch "ui: a refused save is not drawn as a warning" \
    "cargo test -p craftkit-web --profile ci a_failed_save_is_drawn_as_a_warning_and_a_successful_one_is_not"

# The panel did not know that a plan had been reopened, so the line about the
# browser's slot said there was nothing in it. §112.
swap apps/craftkit-web/src/main.rs \
    '                    saved: true,
                    note: format!("reopened “{name}” from this browser"),' \
    '                    saved: false,
                    note: format!("reopened “{name}” from this browser"),'
expect_catch "ui: a reopened plan is not reported as saved" \
    "cargo test -p craftkit-web --profile ci a_stashed_plan_reopens_unchanged"

# ...and the panel ignored its own flag when it drew, so the line said the
# browser was empty whatever the panel knew. §112.
swap apps/craftkit-web/src/main.rs \
    '            let lines = storage_lines(&self.storage);' \
    '            let lines = storage_lines(&Storage { saved: false, ..self.storage.clone() });'
expect_catch "ui: the browser panel ignores what it knows about the stash" \
    "cargo test -p craftkit-web --profile ci the_panels_outcome_is_not_the_shells_outcome"

# The crash reports were written for an agent to read, and the front end whose
# reader is an agent could not read them. §113.
swap crates/mcp/src/tools.rs \
    '        name: "craftkit_crashes",' \
    '        name: "craftkit_crashs",'
expect_catch "mcp: the crash reports are published under a name nobody calls" \
    "cargo test -p craftkit-mcp --profile ci the_crash_reports_are_a_tool"

# The tool ignored the action it was given, so every request was a list. §113.
swap crates/mcp/src/tools.rs \
    '    let action = arguments.get("action").and_then(Json::as_str).unwrap_or("list");' \
    '    let action = "list";'
expect_catch "mcp: the crash tool ignores the action it is given" \
    "cargo test -p craftkit-mcp --profile ci a_crash_action_that_does_not_exist_is_refused"

# The shared dispatcher turned a missing id into a different refusal, so the
# two front ends would not have agreed about it. §113.
swap crates/engine/src/crash.rs \
    '        "show" => {
            let id = id.ok_or(ActionError::MissingId)?;' \
    '        "show" => {
            let id = id.unwrap_or_default();'
expect_catch "engine: a missing report id is not the refusal it should be" \
    "cargo test -p craftkit-engine --profile ci the_crash_actions_are_one_rule"

# ...and the control channel did not ask the engine at all. §113.
swap crates/control/src/server.rs \
    '    match craftkit_engine::crash::action_in(dir, action, id, &from) {' \
    '    match craftkit_engine::crash::action_in(dir, "list", id, &from) {'
expect_catch "control: the crash actions are not the engine's" \
    "cargo test -p craftkit-control --profile ci the_crash_replay_says_whether_it_reproduced_the_document"

# The kernel's two counters were described as the ones the engine tells its
# failures apart with, and the engine uses one of them for a different
# question. The corrected prose says what `guarded()` actually answers: "is
# this thread inside a guarded call?" -- and outside one, the answer is no.
# §114.
swap crates/kernel/src/lib.rs \
    'pub fn guarded() -> bool {
    DEPTH.with(|d| d.get() > 0)
}' \
    'pub fn guarded() -> bool {
    DEPTH.with(|d| d.get() > 0) || true
}'
expect_catch "kernel: the guard depth never unwinds" \
    "cargo test -p craftkit-kernel --profile ci depth_is_balanced_across_nesting_and_early_return"

# The timeline was the design's DURATION -- how many frames the design is
# long -- while the value it held was an absolute frame. A design that starts
# one second in has frames 24..=120 and a duration of 96, so the handle was
# drawn a quarter of the way along the rail and the last twenty-four frames
# could not be reached at all. §115.
swap crates/ui/src/shell.rs \
    '            egui::Slider::new(&mut frame, frames.clone())' \
    '            egui::Slider::new(&mut frame, 0..=meta.duration().frames_i64(rate).max(1))'
expect_catch "ui: the timeline is the duration again" \
    "cargo test -p craftkit-ui --profile ci the_timeline_can_reach_both_ends_of_the_design"

# ...and the model's own answer was the duration too, so every caller that
# asked for the design's frames got the wrong ones. §115.
swap crates/model/src/document.rs \
    '        let start = self.start.frames_i64(rate);' \
    '        let start = 0;'
expect_catch "model: the design's frames are its duration and not its range" \
    "cargo test -p craftkit-model --profile ci the_designs_frames_are_its_range_and_not_its_duration"

# The readout was computed and never drawn, so the timeline said nothing about
# where the playhead was -- and the slider's own formatter never ran, because
# egui only calls it for the value it draws. §115.
swap crates/ui/src/shell.rs \
    '        ui.label(
            RichText::new(playhead_readout(at.frames_i64(rate), at.seconds_f64()))
                .color(tokens.text_dim)
                .small(),
        );' \
    '        let _ = playhead_readout(at.frames_i64(rate), at.seconds_f64());'
expect_catch "ui: the timeline does not say where the playhead is" \
    "cargo test -p craftkit-ui --profile ci the_timeline_says_where_the_playhead_is"

# The readout said the time and not the frame, so the drift between the two --
# the thing §89 is about -- was invisible. §115.
swap crates/ui/src/shell.rs \
    '    format!("frame {frame} · {seconds:.2}s")' \
    '    format!("{seconds:.2}s")'
expect_catch "ui: the readout does not say the frame" \
    "cargo test -p craftkit-ui --profile ci the_playhead_readout_says_the_frame_and_the_time"

# A range that moved under the playhead left it outside the design, where
# there is nothing to see and where the timeline draws a handle at a place
# the playhead is not. §115.
swap crates/engine/src/session.rs \
    '            let was = self.at;
            self.clamp_playhead_into_range();
            if self.at != was {
                self.recompute();
            }' \
    '            let _ = self.at;'
expect_catch "engine: a range that moves under the playhead leaves it there" \
    "cargo test -p craftkit-engine --profile ci a_range_that_moves_under_the_playhead_takes_the_playhead_with_it"

# ...and a document that arrived from outside with its own range did the same
# thing to the playhead. §115.
swap crates/engine/src/session.rs \
    '        self.evaluator.invalidate();
        self.clamp_playhead_into_range();
        self.recompute();' \
    '        self.evaluator.invalidate();
        self.recompute();'
expect_catch "engine: a document that arrives leaves the playhead outside it" \
    "cargo test -p craftkit-engine --profile ci a_document_that_arrives_with_its_own_range_takes_the_playhead_with_it"

# The command moved the playhead and did not say so: a command that changed
# two things and mentioned one lies by omission. §115.
swap crates/engine/src/commands.rs \
    '        let message = if at == was_at {' \
    '        let message = if at != was_at {'
expect_catch "engine: the range command does not say that it moved the playhead" \
    "cargo test -p craftkit-engine --profile ci the_range_command_says_when_it_moved_the_playhead"

# The catalogue marked the id as required, so the schema built from it told a
# client it could not use the name -- which the command accepts, and which the
# tree's own probe table uses for the seek. §116.
swap crates/engine/src/command.rs \
    '                if let Some(alternative) = p.alternative {
                    param["alternative"] = serde_json::json!(alternative);
                }' \
    '                let _ = p.alternative;'
expect_catch "engine: the catalogue does not say which parameter may be given instead" \
    "cargo test -p craftkit-engine --profile ci the_catalogue_says_which_parameter_may_be_given_instead"

# ...and so the schema said nothing about the pair either: the client was
# told it could not use the second spelling, because the first was required.
swap crates/mcp/src/tools.rs \
    '    if !pairs.is_empty() {
        schema["allOf"] = Json::Array(pairs);
    }' \
    '    let _ = pairs;'
expect_catch "mcp: the schema does not say that a parameter has an alternative" \
    "cargo test -p craftkit-mcp --profile ci a_parameter_with_an_alternative_is_one_of_the_two"

# The node parameter's type said "a number or the node's name", and the reader
# refuses a name in it -- the name is the parameter's alternative. §116.
swap crates/mcp/src/tools.rs \
    '            map.insert("type".to_string(), json!("number"));
            map.insert(
                "description".to_string(),
                json!(format!("{help} A whole number, or use `node_name`.")),
            );' \
    '            map.insert("type".to_string(), json!(["number", "string"]));
            map.insert(
                "description".to_string(),
                json!(format!("{help} A whole number, or use `node_name`.")),
            );'
expect_catch "mcp: the node parameter says it takes a name again" \
    "cargo test -p craftkit-mcp --profile ci the_node_parameter_is_a_whole_number"

# The reader stopped resolving the name, so the alternative the catalogue
# offers was a name nothing answered to. §116.
swap crates/engine/src/params.rs \
    '            .find_by_name(&name)' \
    '            .find_by_name("")'
expect_catch "engine: a node cannot be addressed by its name" \
    "cargo test -p craftkit-engine --profile ci a_node_may_be_addressed_by_its_name"

# The catalogue's plain listing dropped the alternative, so a reader of it
# could not tell that one of the two spellings would do. §116.
swap apps/craftkit-cli/src/cli.rs \
    '        let names = group.join("|");' \
    '        let names = group.first().cloned().unwrap_or_default();'
expect_catch "cli: the catalogue's listing does not name the alternative" \
    "cargo test -p craftkit-cli --profile ci a_parameter_with_an_alternative_says_which_one"

# The keyframes' times were in the document, in the file and in three
# commands' parameters -- and no reply said when they were. §117.
swap crates/engine/src/commands.rs \
    '                serde_json::json!({
                    "time": k.time.raw(),
                    "seconds": k.time.seconds_f64(),
                    "value": k.value,
                    "ease": k.ease.name(),
                })' \
    '                serde_json::json!({
                    "time": k.time.raw(),
                })'
expect_catch "engine: the listing does not say when the keyframes are" \
    "cargo test -p craftkit-engine --profile ci the_keyframes_of_a_property_can_be_listed"

# ...and the message -- what `craftkit-cli run` prints and what the status bar
# shows -- did not say it either. §117.
swap crates/engine/src/commands.rs \
    '            format!("{} keyframe(s) on `{path}`: {}", keys.len(), key_times_line(prop.keys.keys())),' \
    '            format!("{} keyframe(s) on `{path}`", keys.len()),'
expect_catch "engine: the listing's message does not say when the keyframes are" \
    "cargo test -p craftkit-engine --profile ci the_listing_says_when_the_keyframes_are"

# The panel said how many keyframes a property had and not when any of them
# was. §117.
swap crates/ui/src/shell.rs \
    '                ui.label(RichText::new(keyframe_times(prop.keys.keys())).color(tokens.text_dim).small());' \
    '                let _ = keyframe_times(prop.keys.keys());'
expect_catch "ui: the panel does not say when the keyframes are" \
    "cargo test -p craftkit-ui --profile ci the_panel_says_when_the_keyframes_are"

# The toolbar ran every command with an empty parameter object, so seventeen
# of its twenty-one buttons could only be refused. §118.
swap crates/ui/src/shell.rs \
    '                        self.start(spec.id);' \
    '                        self.run(spec.id, serde_json::json!({}));'
expect_catch "ui: the toolbar runs a command with no parameters" \
    "cargo test -p craftkit-ui --profile ci no_toolbar_button_is_refused_for_wanting_a_parameter"

# The engine stopped saying what a command asks a person for, so the form
# had nothing to show. §118.
swap crates/engine/src/command.rs \
    '        self.asks.iter().filter_map(|name| self.params.iter().find(|p| p.name == *name)).collect()' \
    '        Vec::new()'
expect_catch "engine: the catalogue no longer says what a command asks for" \
    "cargo test -p craftkit-ui --profile ci no_toolbar_button_is_refused_for_wanting_a_parameter"

# A command that needs a parameter stopped asking for it, so the form could
# not ask for what the command cannot do without. §118.
swap crates/engine/src/commands.rs \
    '    asks: &["name"],' \
    '    asks: &[],'
expect_catch "engine: a command that needs a parameter does not ask for it" \
    "cargo test -p craftkit-engine --profile ci every_parameter_a_command_needs_is_one_it_asks_for"

# The form's OK button stopped running the command, so the form asked a
# question and never did anything with the answer. §118.
swap crates/ui/src/shell.rs \
    '                    if ok.clicked() {
                        submit = true;
                    }' \
    '                    if ok.clicked() {
                        submit = false;
                    }'
expect_catch "ui: the form's OK button does not run the command" \
    "cargo test -p craftkit-ui --profile ci the_form_runs_the_command_when_it_is_filled_in"

# A node's name went to `node` instead of `node_name`, so the two spellings
# of one parameter stopped being two spellings. §116, §118.
swap crates/engine/src/params.rs \
    '                    Some(alternative) => Ok(Typed { name: alternative, value: serde_json::json!(trimmed) }),' \
    '                    Some(_alternative) => Ok(Typed { name: param.name, value: serde_json::json!(trimmed) }),'
expect_catch "engine: a node's name no longer goes to the name spelling" \
    "cargo test -p craftkit-engine --profile ci a_node_id_and_a_node_name_are_the_same_parameter_in_two_spellings"

# The form stopped being drawn, so the state said there was a form and the
# screen said there was not. §118.
swap crates/ui/src/shell.rs \
    '        // The form is drawn last, so it is on top of the panels it is about.
        self.prompt_window(ctx, tokens);' \
    '        // The form is drawn last, so it is on top of the panels it is about.
        let _ = &self.state.prompt;'
expect_catch "ui: the form is not on the screen" \
    "cargo test -p craftkit-ui --profile ci a_frame_with_a_form_is_not_the_frame_without_one"

# A key outside the design's range changed the design and nothing said so.
# §119.
swap crates/engine/src/commands.rs \
    '            format!("keyed `{path}` at {:.3}s{}", time.seconds_f64(), outside.unwrap_or_default()),' \
    '            format!("keyed `{path}` at {:.3}s", time.seconds_f64()),'
expect_catch "engine: the command does not say the key is outside the design" \
    "cargo test -p craftkit-engine --profile ci a_key_outside_the_design_says_so"

# The check for a key before the design stopped firing. §119.
swap crates/engine/src/commands.rs \
    '        let outside = if time < range.start {' \
    '        let outside = if false {'
expect_catch "engine: a key before the design is not noticed" \
    "cargo test -p craftkit-engine --profile ci a_key_before_the_design_says_so"

# The check for a key past the end of the design stopped firing. §119.
swap crates/eval/src/evaluate.rs \
    '    if last.time > end {' \
    '    if false {'
expect_catch "eval: a key past the end of the design is not a warning" \
    "cargo test -p craftkit-eval --profile ci a_keyframe_outside_the_design_is_a_warning"

# The catalogue said a required pair was optional: two entries that both
# say `required: false` say that neither is needed. §120.
swap crates/engine/src/command.rs \
    '        self.params.iter().filter_map(|p| p.alternative.map(|other| vec![p.name, other])).collect()' \
    '        Vec::new()'
expect_catch "engine: the catalogue no longer says which pairs one of must be given" \
    "cargo test -p craftkit-engine --profile ci the_catalogue_says_which_pairs_one_of_must_be_given"

# The plain listing stopped saying the pair is required. §120.
swap apps/craftkit-cli/src/cli.rs \
    '        return format!("      {names}: {kind} — {help} (one of the two is required)");' \
    '        return format!("      {names}: {kind} — {help}");'
expect_catch "cli: the listing does not say the pair is required" \
    "cargo test -p craftkit-cli --profile ci a_pair_prints_as_one_line_that_says_one_of_the_two_is_required"

# The plain listing printed the second spelling as a line of its own again,
# so the pair was two lines that both said "may be left out". §120.
swap apps/craftkit-cli/src/cli.rs \
    '        if group.is_some_and(|g| g.first().map(String::as_str) != Some(name)) {
            continue;
        }' \
    '        if false {
            continue;
        }'
expect_catch "cli: the listing prints the second spelling as its own line" \
    "cargo test -p craftkit-cli --profile ci a_pair_prints_as_one_line_and_the_second_spelling_does_not_print_its_own"

# The keyframes stopped being marked on the rail, so the timeline showed
# the range and the playhead and nothing else. §121.
swap crates/ui/src/shell.rs \
    '        let marks = keyframe_marks(self.session.document(), self.state.selected, rate);' \
    '        let marks: Vec<KeyMark> = Vec::new();'
expect_catch "ui: the timeline marks no keyframes" \
    "cargo test -p craftkit-ui --profile ci the_timeline_marks_the_keyframes"

# The rail stopped spanning the panel and went back to egui's hundred
# pixels. §121.
swap crates/ui/src/shell.rs \
    '        ui.spacing_mut().slider_width = rail_width;' \
    '        ui.spacing_mut().slider_width = previous_width;'
expect_catch "ui: the timeline's rail is a stub again" \
    "cargo test -p craftkit-ui --profile ci the_timeline_rail_follows_the_panel"

# A mark stopped being at the frame it names: every keyframe was drawn in
# the middle of the rail. §121.
swap crates/ui/src/shell.rs \
    '    let share = ((frame - start) as f64 / (end - start) as f64).clamp(0.0, 1.0);' \
    '    let share = 0.5;'
expect_catch "ui: a keyframe's mark is not at its frame" \
    "cargo test -p craftkit-ui --profile ci a_keyframe_mark_is_where_the_playhead_goes_when_it_is_clicked"

# The guard that keeps a key outside the design off the rail stopped
# firing, so a key the playhead can never reach was drawn on the rail as
# if it were inside the design. §121.
swap crates/ui/src/shell.rs \
    '            if mark.frame < *frames.start() || mark.frame > *frames.end() {' \
    '            if mark.frame < *frames.start() && mark.frame > *frames.end() {'
expect_catch "ui: a key outside the design is drawn on the rail" \
    "cargo test -p craftkit-ui --profile ci a_key_outside_the_design_has_no_mark_on_the_rail"

# The model counted a name's occurrences by scanning the siblings it had
# already seen, so listing a node's properties was quadratic. §122.
swap crates/model/src/property.rs \
    '    let count = seen.get(&key).copied().unwrap_or(0) + 1;' \
    '    let count = seen.keys().filter(|k| k.eq_ignore_ascii_case(&label)).count() + 1;'
expect_catch "model: the listing counts the siblings the slow way again" \
    "cargo test -p craftkit-model --profile ci listing_a_node_does_not_cost_more_than_listing_a_smaller_one"

# The count forgot the siblings it had already seen, so a duplicated name was
# published as a first and resolved to the wrong property. §122.
swap crates/model/src/property.rs \
    '    let count = seen.get(&key).copied().unwrap_or(0) + 1;' \
    '    let count = 1;'
expect_catch "model: the listing forgets a sibling when counting" \
    "cargo test -p craftkit-model --profile ci a_listing_of_many_siblings_numbers_the_duplicates"

# The marks stopped looking inside a group, so a keyframe in one was on no
# screen at all. §122.
swap crates/ui/src/shell.rs \
    '            craftkit_model::PropNode::Group(group) => collect_keys(group, selected, rate, out),' \
    '            craftkit_model::PropNode::Group(_group) => {}'
expect_catch "ui: the marks skip the keyframes inside a group" \
    "cargo test -p craftkit-ui --profile ci a_keyframe_inside_a_group_is_marked_on_the_rail"

# The properties panel copied the selected node's whole property tree on every
# frame — every property, with every keyframe's value — to draw a path, a
# value, a count and the first eight times. §123.
swap crates/ui/src/shell.rs \
    '        let rows = node.props.flatten();' \
    '        let rows: Vec<(String, craftkit_model::Property)> = node.props.flatten().into_iter().map(|(path, prop)| (path.to_string(), prop.clone())).collect();'
expect_catch "ui: the properties panel copies the properties again" \
    "cargo test -p craftkit-ui --profile ci the_properties_panel_does_not_copy_the_keyframes_it_does_not_draw"

# The panel copied the keyframes to draw their times, and the copy took every
# keyframe's value with it — a value can be 64 KiB of text. §123.
swap crates/ui/src/shell.rs \
    '                ui.label(RichText::new(keyframe_times(prop.keys.keys())).color(tokens.text_dim).small());' \
    '                let keys: Vec<craftkit_model::Key> = prop.keys.keys().to_vec();
                ui.label(RichText::new(keyframe_times(&keys)).color(tokens.text_dim).small());'
expect_catch "ui: the properties panel copies the keyframes to draw their times" \
    "cargo test -p craftkit-ui --profile ci the_properties_panel_does_not_copy_the_keyframe_values"

# The panel published a property's label instead of its path, so a property
# inside a group was published under a name no command could address. §123.
swap crates/ui/src/shell.rs \
    '            let path = path.to_string();' \
    '            let path = prop.label.clone();'
expect_catch "ui: the panel publishes a property's label instead of its path" \
    "cargo test -p craftkit-ui --profile ci the_panel_publishes_one_editor_for_each_property_and_no_other"

# The model handed a locked property out without asking about the lock, so
# five of the six commands that write a property changed it anyway — and one
# of them changed its value. §124.
swap crates/model/src/property.rs \
    '        if prop.locked {
            return Err(CoreError::BadData(format!("`{path}` is locked")));
        }
        Ok(Some(prop))' \
    '        Ok(Some(prop))'
expect_catch "model: the model hands a locked property out without asking" \
    "cargo test -p craftkit-engine --profile ci a_locked_property_may_not_be_written"

# The panel published a locked property's editor as an ordinary one, so an
# agent asking "can this be clicked?" was told it could. §124.
swap crates/ui/src/shell.rs \
    '            let enabled = !prop.locked;' \
    '            let enabled = true;'
expect_catch "ui: the panel publishes a locked property's editor as enabled" \
    "cargo test -p craftkit-ui --profile ci a_locked_property_s_editor_is_published_as_disabled"

# The panel drew a locked property's editor in full colour, which is what the
# model says it does not do. §124.
swap crates/ui/src/shell.rs \
    '                        let response = ui.add_enabled(
                            enabled,
                            egui::Slider::new(&mut value, -100.0..=100.0)
                                .text(path.clone())
                                .clamping(egui::SliderClamping::Never),
                        );' \
    '                        let response = ui.add(
                            egui::Slider::new(&mut value, -100.0..=100.0)
                                .text(path.clone())
                                .clamping(egui::SliderClamping::Never),
                        );'
expect_catch "ui: the panel draws a locked property's editor in full colour" \
    "cargo test -p craftkit-ui --profile ci a_locked_property_is_drawn_greyed"

# The properties panel had no scroll area at all, so a node with more
# properties than the panel is tall put the rest of them below the panel's
# bottom edge, where nobody could see them and nobody could reach them. §126.
#
# The anchor carries the line above it, because the node list's scroll area
# says the same thing (§128) and `swap` replaces the **first** match.
swap crates/ui/src/shell.rs \
    '        let mut automation = std::mem::take(&mut self.automation);
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {' \
    '        let mut automation = std::mem::take(&mut self.automation);
        ScrollArea::neither().show(ui, |ui| {'
expect_catch "ui: the properties panel's rows do not scroll" \
    "cargo test -p craftkit-ui --profile ci the_properties_panel_scrolls_to_the_properties_that_do_not_fit"

# egui's scroll area shrinks to the width of its content unless it is told not
# to, and a scroll area narrower than the panel answers the wheel only over
# that narrower strip — so the wheel did not reach the list from the panel's
# edge. §126.
#
# The anchor carries the line above it, because the node list's scroll area
# says the same thing (§128) and `swap` replaces the **first** match.
swap crates/ui/src/shell.rs \
    '        let mut automation = std::mem::take(&mut self.automation);
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {' \
    '        let mut automation = std::mem::take(&mut self.automation);
        ScrollArea::vertical().show(ui, |ui| {'
expect_catch "ui: the properties panel's scroll area shrinks to its content" \
    "cargo test -p craftkit-ui --profile ci the_wheel_scrolls_the_list_from_across_the_panel"

# A scroll area takes the rest of the panel, so a control drawn after it is
# laid out below the panel's bottom edge. The key editor was drawn after the
# list, and a node with many properties put it out of sight. §126.
swap crates/ui/src/shell.rs \
    '        Shell::key_editor(&mut self.state, &mut self.automation, self.session, ui, tokens, id, &mut actions);
        ui.separator();' \
    '        ui.separator();'
swap crates/ui/src/shell.rs \
    '        self.automation = automation;

        for (path, value) in edits {' \
    '        self.automation = automation;
        Shell::key_editor(&mut self.state, &mut self.automation, self.session, ui, tokens, id, &mut actions);

        for (path, value) in edits {'
expect_catch "ui: the properties panel draws the key editor after the list" \
    "cargo test -p craftkit-ui --profile ci the_key_editor_stays_on_screen_when_the_node_has_many_properties"

# The plan answers "what is this node's level?" by **scanning** its own
# nodes, so asking it about every node in the list was a scan per node —
# quadratic. At the model's own limit of ten thousand nodes that was 43.7 ms
# of collecting, every frame. The levels are found **once** now. §127.
swap crates/ui/src/shell.rs \
    '            level: levels.get(&n.id).copied().unwrap_or(0.0),' \
    '            level: match plan {
                Some(p) => p.state(n.id).map(|s| s.level_out).unwrap_or(0.0),
                None => levels.get(&n.id).copied().unwrap_or(0.0),
            },'
expect_catch "ui: the node list asks the plan once per node again" \
    "cargo test -p craftkit-ui --profile ci the_node_list_does_not_scan_the_plan_once_per_node"

# The rows carry the level the plan resolved. A row that said zero for every
# node would draw a bar with nothing in it and look perfectly right. §127.
swap crates/ui/src/shell.rs \
    '    let levels: HashMap<NodeId, f64> = match plan {
        Some(p) => p.nodes.iter().map(|s| (s.id, s.level_out)).collect(),
        None => HashMap::new(),
    };' \
    '    let levels: HashMap<NodeId, f64> = HashMap::new();'
expect_catch "ui: the node list shows every level as zero" \
    "cargo test -p craftkit-ui --profile ci the_node_list_carries_the_level_the_plan_resolved"

# The node list's scroll area had the same defect the properties panel's had
# and was never told about it: egui shrinks a scroll area to the width of its
# content, and a scroll area narrower than the panel answers the wheel only
# over that narrower strip — so the wheel did nothing over the panel's
# right-hand margin. §128.
swap crates/ui/src/shell.rs \
    '        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            for row in rows {' \
    '        ScrollArea::vertical().show(ui, |ui| {
            for row in rows {'
expect_catch "ui: the node list's scroll area shrinks to its content" \
    "cargo test -p craftkit-ui --profile ci the_wheel_scrolls_the_node_list_from_across_the_panel"

# The evaluator's cache kept a node's state and dropped the problems that
# node reported, so a reused node reported nothing and the plan said the
# design was fine - the editor's problems panel emptied itself the moment
# anything was recomputed. §129.
swap crates/eval/src/evaluate.rs \
    '                        for problem in &e.diagnostics {
                            diagnostics.push(problem.clone());
                        }
' \
    ''
expect_catch "eval: a reused node repeats its problems" \
    "cargo test -p craftkit-eval --profile ci a_reused_node_still_reports_its_problem"

# The rule that there is no second path, and the engine's own public API.
# `Session::recompute` re-evaluates the document with no command, no trace
# step and no log entry - it had no caller at all, and it was public. §135.
swap crates/engine/src/session.rs \
    '    pub(crate) fn recompute(&mut self) {' \
    '    pub fn recompute(&mut self) {'
expect_catch "engine: the session offers a second path to the plan" \
    "cargo xtask lists"

# The other road: `Session::seek_to` moves the playhead without the clamp
# the registry applies, and the crash replay is what calls it. §135.
swap crates/engine/src/session.rs \
    '    pub(crate) fn seek_to(&mut self, at: i64) {' \
    '    pub fn seek_to(&mut self, at: i64) {'
expect_catch "engine: the session offers a second path to the playhead" \
    "cargo xtask lists"

# And the clamp itself: the playhead is a place **in** the design (§115),
# and the roads that do not go through the registry have to keep it there.
swap crates/engine/src/session.rs \
    '        self.clamp_playhead_into_range();
        self.recompute();
    }

    /// Move the playhead to a tick.' \
    '        self.recompute();
    }

    /// Move the playhead to a tick.'
expect_catch "engine: the playhead can leave the design" \
    "cargo test -p craftkit-engine --profile ci the_playhead_cannot_leave_the_design"

# The same clamp, on the road a user walks: a crash replay. The report
# records the playhead at crash time and the design at the start of the
# trace, and the two disagree whenever the range changed during the trace.
swap crates/engine/src/session.rs \
    '        self.clamp_playhead_into_range();
        self.recompute();
    }

    /// Move the playhead to a tick.' \
    '        self.recompute();
    }

    /// Move the playhead to a tick.'
expect_catch "engine: a crash replay puts the playhead outside the design" \
    "cargo test -p craftkit-engine --profile ci a_crash_replay_does_not_put_the_playhead_outside_the_design"

# The other half of "a panel never writes to the document": the writers are
# public, because the host opens designs with them (§18), so the shell could
# have replaced the design under the user's hands. It never did - and the
# gate is what makes that a fact rather than a habit. §135.
swap crates/ui/src/shell.rs \
    '        let rows = node_rows(self.session.document(), self.session.plan());' \
    '        let rows = node_rows(self.session.document(), self.session.plan());
        self.session.load_document(self.session.document().clone());'
expect_catch "ui: the shell puts a document into the session" \
    "cargo xtask lists"

# A parameter is never indexed, and the one door out - the raw JSON - has two
# homes. A third file that opens it decides the shape of a parameter by
# indexing it, which is either a panic or a silent wrong answer. §136.
swap crates/engine/src/commands.rs \
    '        if !p.has("start") && !p.has("end") && !p.has("fps") {' \
    '        if p.raw_json("start").is_none() && !p.has("end") && !p.has("fps") {'
expect_catch "core: a third file reads the raw parameter JSON" \
    "cargo xtask lists"

# And the setting in clippy.toml that re-enabled a lint nobody denies: it
# enforced nothing, and it made the file look as though it held down the rule
# above. §136.
swap clippy.toml \
    'allow-panic-in-tests = true' \
    'allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true'
expect_catch "clippy: a setting re-enables a lint nobody denies" \
    "cargo xtask lists"

# The id counter, which used to be saturating - and a saturating counter
# hands out its last id for ever. A file carrying that id made every later
# "node.add" mint a node that shared an id with one already there, and a
# document with two nodes that share an id is one the file format refuses.
# §137.
swap crates/core/src/ids.rs \
    '        if self.0 >= Self::MAX {
            return None;
        }' \
    '        if false {
            return None;
        }'
expect_catch "model: the id counter hands out the same id twice" \
    "cargo test -p craftkit-core --profile ci the_counter_never_hands_out_the_same_id_twice"

# And the same rule from the other side: a ceiling that is off by one hands
# out the last id once, and the file that carries it gets a duplicate.
swap crates/core/src/ids.rs \
    '        if self.0 >= Self::MAX {
            return None;
        }' \
    '        if self.0 > Self::MAX {
            return None;
        }'
expect_catch "model: the id counter ceiling is off by one" \
    "cargo test -p craftkit-model --profile ci a_document_whose_ids_reached_the_ceiling_refuses_a_new_node"

# The third half of "a panel never writes to the document, never calls the
# evaluator, never caches a value the evaluator computes": the shell's state
# kept a number between frames. A number in the state is a number the design
# said, and the design can change - so the copy goes stale and nobody can
# tell. §138.
swap crates/ui/src/shell.rs \
    '    /// The last thing that happened, shown in the status bar.
    pub status: String,' \
    '    /// The last thing that happened, shown in the status bar.
    pub status: String,
    /// A level the shell kept between frames.
    pub cached_level: f64,'
expect_catch "ui: the shell caches a value the evaluator computes" \
    "cargo xtask lists"

# And the same state holding the evaluator's own answer: a plan copied into
# the shell goes stale the moment the design changes.
swap crates/ui/src/shell.rs \
    '    /// The last thing that happened, shown in the status bar.
    pub status: String,' \
    '    /// The last thing that happened, shown in the status bar.
    pub status: String,
    /// A copy of the plan, kept between frames.
    pub cached_plan: Option<craftkit_eval::Plan>,'
expect_catch "ui: the shell keeps a copy of the plan" \
    "cargo xtask lists"

# The playbook's rule 4: every parameter a command reads is declared in its
# `params`. A command that reads one it does not declare cannot be given the
# value it needs - the engine refuses an undeclared parameter - and the
# command is dead. §139.
swap crates/engine/src/commands.rs \
    '        let name = required_text(p, "name")?;
        check_name(&name)?;
        let changed = s.doc_mut().set_name(name.clone());' \
    '        let name = required_text(p, "title")?;
        check_name(&name)?;
        let changed = s.doc_mut().set_name(name.clone());'
expect_catch "engine: a command reads a parameter it does not declare" \
    "cargo xtask lists"

# And the other half, which is the silent one: a command that declares a
# parameter and ignores it. The caller passes the value and the command does
# something else.
swap crates/engine/src/commands.rs \
    '        let muted = match p.bool("muted")? {
            Some(b) => b,
            None => !node_mut(s, id, NODE_MUTE.id)?.muted,
        };' \
    '        let _asked = p.bool("muted")?;
        let muted = !node_mut(s, id, NODE_MUTE.id)?.muted;'
expect_catch "engine: a command declares a parameter it never reads" \
    "cargo xtask lists"

# The recipe for adding a command quotes the command it wants you to copy.
# The quote had drifted: it was missing a field, so it did not compile, and
# it taught the two mistakes sections 93 and 116 were about. The block now
# says it is exact, and the gate compares statics as well as functions and
# structs. §140.
swap docs/recipes/add-a-command.md \
    '        ParamDoc::either("node", Doc::NodeRef, "The node'"'"'s id, or its name.", "node_name"),' \
    '        ParamDoc::req("node", Doc::NodeRef, "The node'"'"'s id."),'
expect_catch "prose: a quoted command spec is not the one in the code" \
    "cargo xtask lists"

# The design file's indentation grew with the depth of the design, so a file
# was up to 9.9 times the design it held -- and the tree's file cap was
# smaller than the file some designs inside the model's own budget needed.
# The tree could hold designs it could not save. §141.
#
# The bound on the indentation is the whole of the fix: take it away and the
# file is ten times the design again, and a design the model accepts no
# longer fits in a file.
swap crates/core/src/writer.rs \
    'pub const MAX_INDENT_DEPTH: usize = 4;' \
    'pub const MAX_INDENT_DEPTH: usize = 64;'
expect_catch "io: the writer indents as deep as the design goes" \
    "cargo test -p craftkit-io --profile ci a_designs_file_is_bounded_by_the_design_it_holds"

# The format's expansion is the number that says how many bytes of text one
# byte of design takes, and the file's cap is the design's budget times it.
# Understate the expansion and the cap is too small for a design the model
# accepts: the tree holds a design it cannot save.
swap crates/io/src/format.rs \
    'pub const FILE_EXPANSION: usize = 2;' \
    'pub const FILE_EXPANSION: usize = 1;'
expect_catch "io: the format's expansion is understated" \
    "cargo test -p craftkit-io --profile ci a_document_the_model_accepts_can_be_written_and_read_back"

# And the cap itself has to be *derived*. A literal that happens to be the
# right number today is a number that can be wrong tomorrow, and the test in
# the io crate cannot see it: the literal and the derivation have the same
# value. That is what the gate is for.
swap crates/io/src/format.rs \
    'pub const MAX_FILE_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * FILE_EXPANSION;' \
    'pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;'
expect_catch "io: the file's cap is a number somebody chose" \
    "cargo xtask lists"

# The crash report wrote its files with serde_json's pretty printer -- the
# tree's layout without the bound on the indentation -- so a report of a
# design the model accepts was 73.5 MiB of text, and the crash reader refuses
# anything past 64 MiB. The app wrote crash reports it could not read back.
# §142.
swap crates/engine/src/crash.rs \
    '    craftkit_core::writer::write_json(&mut out, value).map_err(|e| format!("could not serialize: {e}"))?;' \
    '    let text = serde_json::to_string_pretty(value).map_err(|e| format!("could not serialize: {e}"))?;
    out.extend_from_slice(text.as_bytes());'
expect_catch "engine: the crash report is written with an unbounded writer" \
    "cargo test -p craftkit-engine --profile ci a_crash_report_of_a_big_design_can_be_read_back"

# And a writer of its own that is bounded: the report would fit, and the
# crash report's own test would not notice -- but the design in the crash
# folder would not be the design on disk, and the gate says so.
swap crates/engine/src/crash.rs \
    '    craftkit_core::writer::write_json(&mut out, value).map_err(|e| format!("could not serialize: {e}"))?;' \
    '    let text = serde_json::to_string(value).map_err(|e| format!("could not serialize: {e}"))?;
    out.extend_from_slice(text.as_bytes());'
expect_catch "engine: the crash report is written with a writer of its own" \
    "cargo xtask lists"

# The recipe for adding a control-channel method told an agent to add a
# `Method::Render` variant -- a variant the tree has never had -- and to move
# `ui.render` out of the group of spellings that reach `Method::Ui`. Applied to
# the tree, that splits the `ui.*` family and `ui.render` stops reaching the
# host that draws the frame. The prose named a member the type does not have,
# and nothing checked that. §143.
swap docs/recipes/add-a-control-channel-method.md \
    'Method::Ui => match ui {' \
    'Method::Render => "ui.render",
Method::Ui => match ui {'
expect_catch "prose: the recipe names a method the tree does not have" \
    "cargo xtask lists"

# And the same defect somewhere else, so the check is about the tree's prose
# and not about one document: a limit named by a path the type does not have.
# `craftkit_engine::history::MAX_ENTRIES` is a module-level constant; there is
# no `History::MAX_ENTRIES`, and the engine's log has a `MAX_ENTRIES` of its
# own with a different value. §143.
swap docs/architecture.md \
    '`craftkit_engine::history::MAX_ENTRIES`' \
    '`History::MAX_ENTRIES`'
expect_catch "prose: a limit is named by a path the type does not have" \
    "cargo xtask lists"

# The model's limit table said a document's budget was "the number that says
# what the process actually uses", and the process holds four documents: the
# one being edited, the base the command trace starts from, and the undo and
# redo stacks. Measured: 118.12 MiB of documents and 102.67 MiB of resident
# memory, 3.18 times the number the table named. The history's own budget was
# a literal that happened to be the design file's cap -- a cap section 141 had
# already derived. Section 144.
swap crates/engine/src/history.rs \
    'pub const MAX_HISTORY_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION;' \
    'pub const MAX_HISTORY_BYTES: usize = 64 * 1024 * 1024;'
expect_catch "engine: the history's budget is a number somebody chose" \
    "cargo xtask lists"

# And the session's budget has to cover the base the trace starts from, not
# only the document being edited. A budget that forgot it would be a fifth
# short -- and this one the runtime test can see, because the numbers move.
swap crates/engine/src/session.rs \
    'pub const SESSION_OWN_DOCUMENTS: usize = 2;' \
    'pub const SESSION_OWN_DOCUMENTS: usize = 1;'
expect_catch "engine: the session's budget forgot the trace's base" \
    "cargo test -p craftkit-engine --profile ci the_session_budget_covers_the_stacks_and_the_documents_it_owns"

# The pattern document for cloning before editing keeps a table of what undoing
# costs. It listed the count of undo entries and the number of nodes in a
# document, and not the bound that actually binds -- the undo and redo stacks'
# 64 MiB -- and the paragraph under it told a reader that "the fix is to raise
# MAX_ENTRIES and add a size cap". Section 52 had already added the size cap,
# thirteen sections earlier, and raising the count changes nothing for a
# document over half a megabyte: measured, a design at the model's limit keeps
# two snapshots, not a hundred and twenty-eight. Section 145.
#
# The first mutation takes a bound out of the tree's documents. It is named in
# exactly one place, and a bound named in exactly one place is one edit away
# from being named in none.
swap docs/architecture.md \
    ', `MAX_PATH_BYTES` `MAX_PATH_SEGMENTS * (MAX_NAME + 1) - 1`' \
    ','
expect_catch "prose: a bound the tree has is not named" \
    "cargo xtask lists"

# And the advice itself: a document must not tell a reader to add a bound the
# tree already has. The phrase is searched for with the whitespace collapsed,
# because a document that wraps it across two lines is the same sentence.
swap docs/patterns/clone-before-edit.md \
    '`craftkit_engine::history::HISTORY_EXPANSION` — not to raise `MAX_ENTRIES`,' \
    '`craftkit_engine::history::HISTORY_EXPANSION` and add a size cap — not to raise `MAX_ENTRIES`,'
expect_catch "prose: the pattern tells a reader to add a size cap" \
    "cargo xtask lists"

# The gate that compares an exact block with the code it names read
# functions, structs and statics, and nothing else. The tree's time pattern
# quotes a const in an exact block, and that const was invisible to the gate:
# measured, the source could change the number and the document could say any
# number at all, and the gate stayed green. An enum was worse in both
# directions -- quoted beside other code it was not compared, and quoted on
# its own it was reported as "quotes no code", which is a false alarm against
# a correct document. Section 146.
#
# The first mutation changes the number the document quotes. The gate has to
# notice that the document is no longer the code.
swap docs/patterns/exact-time-not-float-seconds.md \
    'pub const TICKS_PER_SECOND: i128 = 254_016_000_000;' \
    'pub const TICKS_PER_SECOND: i128 = 254_016_000_001;'
expect_catch "prose: a const in an exact block drifts" \
    "cargo xtask lists"

# The second changes the enum the pattern quotes, the way a variant is added
# to the source and the document is not told.
swap docs/patterns/everything-is-a-command.md \
    '    Playback,
    History,
    Debug,
}' \
    '    Playback,
    History,
    Debug,
    Audio,
}'
expect_catch "prose: an enum in an exact block drifts" \
    "cargo xtask lists"

# And the third quotes something the gate cannot compare. A block marked
# `exact` promises that it is the code in the file, so an item the gate does
# not compare has to be reported by name -- not passed over in silence.
swap docs/patterns/everything-is-a-command.md \
    'pub enum Area {
    Document,' \
    'impl Area {
    pub fn key(self) -> &'"'"'static str {
        "area"
    }
}

pub enum Area {
    Document,'
expect_catch "prose: an exact block quotes something the gate cannot compare" \
    "cargo xtask lists"

# And the limit the recipe derives by division. The gate that reads a limit in
# prose followed only an expression holding "<<" or "*", so the recipe's claim
# that MAX_JSON_TEXT_BYTES is MAX_JSON_MEMORY_BYTES / JSON_EXPANSION was never
# compared with the code: measured, the divisor could change and the prose
# would keep claiming the old one, and the gate said nothing. Section 147.
#
# The first mutation changes the divisor in the code. The prose still claims
# the old expression, and the gate has to say so.
swap crates/core/src/lib.rs \
    'pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / JSON_EXPANSION;' \
    'pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / 16;'
expect_catch "prose: a limit derived by division is not the constant's value" \
    "cargo xtask lists"

# The second changes the claim in the prose. The code says what it says, and
# the document has to be held to it.
swap docs/recipes/add-a-control-channel-method.md \
    '`MAX_JSON_MEMORY_BYTES / JSON_EXPANSION`' \
    '`MAX_JSON_MEMORY_BYTES / 16`'
expect_catch "prose: a claim of division is not the constant's value" \
    "cargo xtask lists"

# The tree's limits, read out of its source. The tree has the rule that a
# mention in a string is not code -- section 78 wrote code_only for it -- and
# four gates that read the tree's limits did not apply it: they read the
# text. Measured: a design file's cap that stood only inside a string
# satisfied the gate that exists to check that cap; so did the undo stacks'
# budget; and a fabricated constant masked a real drift in the prose, because
# the gate accepts a claim that matches *any* value of that name. Section 148.
#
# The first mutation takes the rule out of the gate that reads the tree's
# constants for the prose.
swap xtask/src/lists.rs \
    '    let code = code_lines_kept(source);
    for line in code.lines() {' \
    '    let code = source.to_string();
    for line in code.lines() {'
expect_catch "gate: the prose reads the tree's limits out of text" \
    "cargo test -p xtask --profile ci a_constant_that_stands_only_in_a_string_is_not_a_constant_of_the_tree"

# The second takes it out of the gate that holds the design file's cap to the
# model's budget.
swap xtask/src/lists.rs \
    '    let code = code_lines_kept(&text);
    let lines: Vec<&str> = code.lines().collect();
    let Some(index) = lines.iter().position(|l| l.trim_start().starts_with("pub const MAX_FILE_BYTES"))' \
    '    let code = text.clone();
    let lines: Vec<&str> = code.lines().collect();
    let Some(index) = lines.iter().position(|l| l.trim_start().starts_with("pub const MAX_FILE_BYTES"))'
expect_catch "gate: the file cap reads the tree's limits out of text" \
    "cargo test -p xtask --profile ci a_limit_that_stands_only_in_a_string_is_not_a_limit"

# The third takes it out of the gate that holds the memory budgets to the
# model's budget.
swap xtask/src/lists.rs \
    '        let code = code_lines_kept(&text);
        let lines: Vec<&str> = code.lines().collect();
        let Some(index) =' \
    '        let code = text.clone();
        let lines: Vec<&str> = code.lines().collect();
        let Some(index) ='
expect_catch "gate: the memory budgets read the tree's limits out of text" \
    "cargo test -p xtask --profile ci a_limit_that_stands_only_in_a_string_is_not_a_limit"

# And the fourth out of the gate that says every bound the tree has is named
# in the tree's prose.
swap xtask/src/lists.rs \
    'for name in the_tree_s_bounds(&code_lines_kept(&text)) {' \
    'for name in the_tree_s_bounds(&text) {'
expect_catch "gate: the byte bounds read the tree's limits out of text" \
    "cargo test -p xtask --profile ci a_bound_that_stands_only_in_a_string_is_not_a_bound_of_the_tree"

# The tree's rule that a mention in a string is not code was written three
# times -- as code_only, as without_comments and as strip_code -- and the
# gates that read a *symbol* or a *module* used none of them, so a rule that
# existed in the tree was not applied to most of the tree. The tree has one
# reader now, code_lines_kept, and every gate that reads the tree's source
# goes through it. Measured, against the gates themselves on a copy of the
# tree: a test that stood only inside a string satisfied a document's claim
# about it; so did one that stood only inside a comment; so did a module;
# so did a limit; and a block comment in front of a constant moved the line
# the gate named by nine. Section 149.
#
# The first takes the rule out of the gate that checks the claims prose
# makes about the tree's own files.
swap xtask/src/lists.rs \
    '    let code = code_lines_kept(source);
    for keyword in ["fn ", "struct ", "enum ", "const ", "static ", "mod ", "trait ", "type "] {' \
    '    let code = source.to_string();
    for keyword in ["fn ", "struct ", "enum ", "const ", "static ", "mod ", "trait ", "type "] {'
expect_catch "gate: the symbol gate reads the tree's source as text" \
    "cargo test -p xtask --profile ci a_symbol_that_stands_only_in_a_string_is_not_a_symbol_of_the_tree"

# The second takes the other half of the rule out of the one reader, so a
# comment is code again.
swap xtask/src/lists.rs \
    "            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line = true;
                out.push(' ');
                out.push(' ');
            }" \
    "            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                out.push(' ');
                out.push(' ');
            }"
expect_catch "gate: the one reader keeps a comment's text" \
    "cargo test -p xtask --profile ci a_symbol_that_stands_only_in_a_comment_is_not_a_symbol_of_the_tree"

# The third takes the rule out of the gate that says every module path in
# prose names a module this tree has.
swap xtask/src/lists.rs \
    '        let code = code_lines_kept(&text);
        for line in code.lines() {' \
    '        let code = text.clone();
        for line in code.lines() {'
expect_catch "gate: the module gate reads the tree's source as text" \
    "cargo test -p xtask --profile ci a_module_that_stands_only_in_a_string_is_not_a_module_of_the_tree"

# The fourth out of the gate that says every limit in the model's table is
# enforced somewhere in the tree.
swap xtask/src/lists.rs \
    '        let body = if path == table { rest.clone() } else { code_lines_kept(&text) };' \
    '        let body = if path == table { rest.clone() } else { text.clone() };'
expect_catch "gate: the limits gate reads the tree's strings as code" \
    "cargo test -p xtask --profile ci a_limit_that_stands_only_in_a_string_is_not_enforced"

# The fifth takes the newline out of a string continuation. A string written
# across two lines is two lines of the file, and the tree's own
# crates/io/src/format.rs has three of them.
swap xtask/src/lists.rs \
    "                blank(&mut out, c);
                escaped = false;" \
    "                out.push(' ');
                escaped = false;"
expect_catch "gate: a string continuation loses the line it ends" \
    "cargo test -p xtask --profile ci a_gate_that_names_a_line_names_the_file_s_own_line"

# The sixth takes the newlines out of a block comment, which moves every
# line after it.
swap xtask/src/lists.rs \
    "            } else if c == '\n' {
                out.push('\n');
            } else {
                blank(&mut out, c);
            }
            continue;
        }
        if in_string {" \
    "            } else {
                out.push(' ');
            }
            continue;
        }
        if in_string {"
expect_catch "gate: a block comment loses the lines it covers" \
    "cargo test -p xtask --profile ci a_gate_that_names_a_line_names_the_file_s_own_line"

# And the seventh grows the tree a second reader, which is the thing the
# one-reader check exists to stop.
swap xtask/src/lists.rs \
    'pub fn code_lines_kept(source: &str) -> String {' \
    'pub fn a_second_code_reader(source: &str) -> String {
    let mut in_string = false;
    let mut out = String::new();
    for c in source.chars() {
        if c == char::from(34) {
            in_string = !in_string;
        }
        out.push('"'"' '"'"');
    }
    out
}

pub fn code_lines_kept(source: &str) -> String {'
expect_catch "gate: the tree grows a second code reader" \
    "cargo test -p xtask --profile ci the_tree_has_one_code_reader"

# A test is code, and it is not enforcement. The gate that says every limit
# in the model's table is enforced somewhere reads the tree's files whole,
# and a test that reads the limit to compare it with another limit enforces
# nothing. Measured, against the gate on a copy of the tree: the limit's one
# production use was replaced by a literal, its only remaining mention was in
# a test, and the gate said green -- while the control, with the limit
# mentioned nowhere, was red. Section 150.
#
# The first takes the cut out of the other files.
swap xtask/src/lists.rs \
    '        let Ok(text) = read_rust(&path, Subject::Production) else { continue };
        let body = if path == table { rest.clone() } else { code_lines_kept(&text) };' \
    '        let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
        let body = if path == table { rest.clone() } else { code_lines_kept(&text) };'
expect_catch "gate: the limits gate reads the tree's tests as enforcement" \
    "cargo test -p xtask --profile ci a_limit_only_a_test_mentions_is_still_caught"

# And the second out of the file the limits table itself lives in.
swap xtask/src/lists.rs \
    '    let source = read_rust(&table, Subject::Production)
        .map_err(|e| format!("could not read the limits table: {e}"))?;
    let code = code_lines_kept(&source);' \
    '    let source = read_rust(&table, Subject::Everything)
        .map_err(|e| format!("could not read the limits table: {e}"))?;
    let code = code_lines_kept(&source);'
expect_catch "gate: the limits gate reads the table file's tests as enforcement" \
    "cargo test -p xtask --profile ci a_limit_only_a_test_mentions_is_still_caught"

# A test is not a third home of the raw parameter reader. The gate that holds
# the reader to two files read the tree's files whole, so a test in a third
# file that called the reader to prove the reader works was reported as a
# third place where the shape of a parameter is decided. Measured: a third
# file whose *test* called the reader was reported, and the same call in
# production code was reported too -- the gate could not tell them apart.
# Section 151.
#
# The anchor names the line after it as well, because the line on its own is
# in three places in this file and `swap` replaces the first of them. It
# happened to be the right one -- the raw reader's gate is the first of the
# three -- and that is luck, not a rule. §184.
swap xtask/src/lists.rs \
    '        let Ok(text) = read_rust(&path, Subject::Production) else { continue };
        // Strings and comments do not count: this file names the reader, and' \
    '        let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
        // Strings and comments do not count: this file names the reader, and'
expect_catch "gate: the raw reader gate counts a test's call as a third home" \
    "cargo test -p xtask --profile ci a_test_that_calls_the_raw_reader_is_not_a_third_home"

# A test's method is not a public path into the design. The gate that says
# the session offers no second path counts every "pub fn" in the session's
# impl block, and a method marked #[cfg(test)] is not in the shipped build.
# Measured: a test-only method in the impl block was reported as "a public
# way to change the design is a second path", while the control, with no such
# method, was green. Section 152.
swap xtask/src/lists.rs \
    '        if trimmed.starts_with("#[cfg(test)]") {
            test_only = true;
            continue;
        }' \
    '        if trimmed.starts_with("#[cfg(test)]") {
            test_only = false;
            continue;
        }'
expect_catch "gate: the session gate counts a test's method as a second path" \
    "cargo test -p xtask --profile ci a_test_only_method_is_not_a_second_path_into_the_design"

# A test that catches a panic is not a boundary. The gate says every file
# with a catch_unwind in it is a declared boundary, and a test may catch a
# panic to assert that it was caught. The gate cut the file at its test
# module, so a test written outside a module was counted as a boundary.
# Measured: a top-level test function that caught a panic was reported, and
# the control, with no such function, was green. Section 153.
#
# The first takes the wider cut out of the boundary gate.
swap xtask/src/lists.rs \
    '        let Ok(text) = read_rust(&path, Subject::Production) else { continue };
        // The tests are not the production code: a test may catch a panic to' \
    '        let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
        // The tests are not the production code: a test may catch a panic to'
expect_catch "gate: the boundary gate counts a test's catch as a boundary" \
    "cargo test -p xtask --profile ci a_test_that_catches_a_panic_is_not_a_boundary"

# And the second makes the cut truncate the file again, which hides
# production code that comes after a test item.
swap xtask/src/lists.rs \
    '        if line.starts_with("mod tests {") || line.starts_with("#[cfg(test)]") {
            // The item ends where its **braces** say, and the braces are
            // found in the code view: a `}` inside a string is not a brace.
            // §160.
            let last = top_level_item_last_line(&code, &starts, &lines, index);
            for _ in index..=last {
                out.push('"'"'\n'"'"');
            }
            index = last + 1;
            continue;
        }' \
    '        if line.starts_with("mod tests {") || line.starts_with("#[cfg(test)]") {
            return out;
        }'
expect_catch "gate: the production cut hides the code after a test item" \
    "cargo test -p xtask --profile ci production_code_after_a_test_item_is_still_production_code"

# A test helper is not a reader. The tree derives its parameter readers from
# its source -- a reader is a function whose signature takes a Params -- and
# a test helper that takes one is such a function. A test helper that shares
# a name with a production reader is worse than useless: the gate believes it
# has no name argument of its own, reads the test's body, and tells the
# commands that they read a parameter the test invented. Measured: a test
# helper named after a production reader, with no name argument of its own,
# made the gate report two commands reading a parameter called "ghost" --
# while the control, with no such helper, was green. Section 154.
# The anchor has to name the **reader list's own** call. An anchor that is
# merely present somewhere in the file is not enough: a two-line anchor was
# ambiguous after §167, and it was disambiguated by adding a third line --
# which moved it to a different function entirely. The mutation then broke a
# gate nobody asked about, the reader list stayed whole, and the case was
# reported MISSED. §167.
#
# §170 added a second gate that reads the tree's source, and its code had the
# same four lines. Both anchors here became ambiguous again, and an ambiguous
# anchor lands on the **first** match -- which is the reader list today and
# might not be tomorrow. So both anchors now end with the reader list's own
# next line, `let is_method = has_word(&function.block, PARAMS_TYPE);`, which
# is in no other function. The probe that reports an anchor in a file twice is
# how it was found, and it is the same rule §167 learned.
swap xtask/src/lists.rs \
    '        let Ok(text) = read_rust(&path, Subject::Production) else { continue };
        let relative = relative_name(root, &path);' \
    '        let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
        let relative = relative_name(root, &path);'
expect_catch "gate: the reader list counts a test's helper as a reader" \
    "cargo test -p xtask --profile ci a_test_helper_is_not_a_reader"

# A comment is not code. The gate that says every parameter a command reads
# is declared derives its two lists -- the readers and the commands -- out of
# the tree's source, and it read them as text. A doc comment that shows a
# reader is the worst shape: the gate believed the reader had no name
# argument of its own, read the comment's body, and told the commands that
# they read a parameter the comment invented. Measured: a doc comment that
# showed a reader made the gate report two commands reading a parameter
# called "ghost" -- and a doc comment that showed a whole command made it
# report a command that does not exist. The controls, with the same prose
# and no signature, were green. Section 155.
swap xtask/src/lists.rs \
    '        for function in rust_functions(&relative, &text, &code) {
            let is_method = has_word(&function.block, PARAMS_TYPE);' \
    '        for function in rust_functions(&relative, &text, &text) {
            let is_method = has_word(&function.block, PARAMS_TYPE);'
expect_catch "gate: the reader list reads a comment as a reader" \
    "cargo test -p xtask --profile ci a_reader_that_stands_only_in_a_comment_is_not_a_reader"

swap xtask/src/lists.rs \
    '    let code = code_lines_kept(&text);
    let mut out = Vec::new();
    let bytes = code.as_bytes();' \
    '    let code = text.clone();
    let mut out = Vec::new();
    let bytes = code.as_bytes();'
expect_catch "gate: the command list reads a comment as a command" \
    "cargo test -p xtask --profile ci a_command_that_stands_only_in_a_comment_is_not_a_command"

# And the code view has to be the same length as the file: the gate reads
# the shape of a thing from the code and its contents from the text, and it
# takes one from the other by position. A block comment's end used to lose a
# byte, and every position after it was off by one.
swap xtask/src/lists.rs \
    "                // Two bytes for two bytes: the view keeps the file's length.
                out.push(' ');
                out.push(' ');" \
    "                out.push(' ');"
expect_catch "gate: the code view loses a byte" \
    "cargo test -p xtask --profile ci the_code_view_is_the_same_length_as_the_file"

# A cut that truncates says "everything after this is not production", and
# that is a claim about the rest of the file that nobody checked. Three
# gates cut the tree's source that way. Measured, against the gates on a
# copy of the tree: a production call to serde_json's pretty printer after
# the test module was invisible to the gate that says the tree writes its
# JSON with one writer; an indented test attribute -- which marks a method
# inside an impl block -- truncated the file there, and the gate reported
# that the design file was not written with the tree's writer, which it is;
# and a test written outside a module was reported by the two gates that
# say the shell never evaluates the document and never writes it. The
# controls, with the same code where the gate was looking, were red. Section
# 156.
swap xtask/src/lists.rs \
    '            let last = top_level_item_last_line(&code, &starts, &lines, index);' \
    '            let last = lines.len() - 1;'
expect_catch "gate: the JSON writer's cut truncates the file" \
    "cargo test -p xtask --profile ci a_writer_after_a_test_item_is_still_a_writer"

# The second case of this section -- the one that made the cut treat an
# indented test attribute as the end of the file -- was **removed** by section
# 160, and it is worth saying why. Section 160 found the end of a test item
# by matching its braces in the code view, and the cut that truncated the file
# at a test marker is gone with it: the mutation that made the cut truncate
# could no longer fail. A case that cannot fail is worse than no case, so it
# was removed rather than kept as a decoration. The rule it protected -- a
# cut that does not truncate the file -- is still protected by the case
# above, which still bites.

swap xtask/src/lists.rs \
    '        let Ok(text) = read_rust(&path, Subject::Production) else { continue };
        // The tests are not the panels: a test may name the evaluator to' \
    '        let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
        // The tests are not the panels: a test may name the evaluator to'
expect_catch "gate: the shell's cut truncates the file" \
    "cargo test -p xtask --profile ci a_test_outside_a_module_is_not_the_shell"

# A test-only method inside an impl block is a test all the same. The cut
# removes the test *items* -- a mod tests block, or a top-level item marked
# cfg(test) -- and it kept the methods, because a method is marked rather
# than put in a module. Measured, against the gates on a copy of the tree: a
# test-only method that names the evaluator was reported by the gate that
# says the shell never evaluates the document; a test-only method named
# after a production reader was read as a reader, and the gate told the
# commands that they read a parameter the test invented; and the limit's one
# production use replaced by a literal, with a test-only method using the
# limit instead, was reported as enforced. Section 157.
swap xtask/src/lists.rs \
    '    let trimmed = line.trim_start();
    trimmed == "#[cfg(test)]" && trimmed.len() != line.len()' \
    '    let _trimmed = line.trim_start();
    false'
expect_catch "gate: the cut keeps a test-only method" \
    "cargo test -p xtask --profile ci a_test_only_method_is_not_a_reader"

# A raw string is a string, and the tree's one code reader did not know it.
# The reader that blanks an ordinary string's contents ends it at the first
# quote inside -- and a raw string has quotes inside it. Measured, against the
# gates on a copy of the tree, with the reader as it was before the fix and
# with the reader as it is after it:
#
#   * a limit whose every production use had been replaced by a literal, and
#     which a raw string merely mentioned, was reported as **enforced** --
#     a limit nobody enforces is a wish, and the gate said it was not. The
#     control, with the same mention in an ordinary string, was reported
#     correctly, before and after.
#   * a raw string with one quote in it, four lines before the writer, was
#     reported by the gate that says the tree writes its JSON with one
#     writer: the design file "is not written with the tree's writer", which
#     is untrue. The reader had ended its ordinary string in the wrong place
#     and read the rest of the file as string contents. The control, with the
#     same text in an ordinary string, was green before and after.
#
# The tree's own raw strings are all in test code, which is why no gate had
# noticed: the defect was latent, and it is measured here rather than
# asserted. Section 158.
swap xtask/src/lists.rs \
    "            'r' if !ends_with_word_byte(&out) => {" \
    "            'r' if ends_with_word_byte(&out) => {"
expect_catch "gate: the reader does not recognise a raw string" \
    "cargo test -p xtask --profile ci a_raw_string_is_a_string"

swap xtask/src/lists.rs \
    "            if seen == hashes {" \
    "            if seen == hashes && hashes == 0 {"
expect_catch "gate: the reader ends a raw string at the first quote in it" \
    "cargo test -p xtask --profile ci a_raw_string_ends_at_its_own_delimiter"

swap xtask/src/lists.rs \
    "    for _ in 0..hashes {
        chars.next();
        out.push('#');
    }
    chars.next();
    out.push('\"');" \
    "    for _ in 0..hashes {
        out.push('#');
    }
    out.push('\"');"
expect_catch "gate: the reader does not consume the raw string's own delimiter" \
    "cargo test -p xtask --profile ci a_raw_string_without_hashes_is_a_raw_string"

swap xtask/src/lists.rs \
    "                let raw = ahead.peek() == Some(&'r');" \
    "                let raw = ahead.peek() == Some(&'R');"
expect_catch "gate: the reader does not recognise a byte string" \
    "cargo test -p xtask --profile ci a_byte_string_is_a_string"

# Three gates that read names read their shape from the text, and a comment
# is a shape. Measured, against the real gates on a copy of the tree, with the
# tree as it stands after section 158 and with section 159's fix in it:
#
#   * a command that stands only inside a block comment made the registry one
#     command longer, and the two documents that say how many commands there
#     are were reported as wrong -- two false positives.
#   * a wire name that stands only inside a block comment made a document's
#     example resolve to a method the control channel does not have: the gate
#     that says a document's example must resolve was satisfied by a comment.
#   * a method that stands only inside a line comment made the gate believe
#     the channel had a method it does not have, and the three documents that
#     list the methods were reported as wrong -- four false positives.
#
# The shape comes from the code and the contents from the text: a command id,
# a wire name and a method name are strings, and there a string *is* the
# subject. Section 159.

swap xtask/src/lists.rs \
    $'        let code = code_lines_kept(&text);
        let lines: Vec<&str> = text.lines().collect();
        for (number, code_line) in code.lines().enumerate() {
            if !code_line.trim_start().starts_with("id:") {
                continue;
            }
            let Some(line) = lines.get(number) else { continue };
' \
    $'        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("id: \\"") else { continue };
'
expect_catch "gate: the registry reads a command id out of a comment" \
    "cargo test -p xtask --profile ci a_command_id_in_a_comment_is_not_a_command_id"

swap xtask/src/lists.rs \
    $'    let code = code_lines_kept(text);
    let lines: Vec<&str> = text.lines().collect();
    for (number, code_line) in code.lines().enumerate() {
        let Some(arrow) = code_line.find("=>") else { continue };
        // Only arms, not the function signature or a `=> return None` arm.
        // The pattern of an arm is a string literal, and in the code view a
        // literal is blanked, so what is left of the pattern is the `|`s
        // between the literals and nothing else.
        if !code_line[..arrow].replace(\'|\', "").trim().is_empty() {
            continue;
        }
        if !code_line[arrow..].contains(marker) {
            continue;
        }
        let Some(line) = lines.get(number) else { continue };
' \
    $'    for line in text.lines() {
        let trimmed = line.trim_start();
        // Only arms, not the function signature or a `=> return None` arm.
        if !trimmed.starts_with(\'"\') {
            continue;
        }
        let Some(arrow) = line.find("=>") else { continue };
        if !line[arrow..].contains(marker) {
            continue;
        }
'
expect_catch "gate: the wire names read an arm out of a comment" \
    "cargo test -p xtask --profile ci a_wire_name_in_a_comment_is_not_a_wire_name"

swap xtask/src/lists.rs \
    $'    let indices = function_line_indices(&code_lines, start);
    let mut names = Vec::new();
    for index in indices {
        let Some(shape) = code_lines.get(index) else { continue };
        let Some(line) = lines.get(index) else { continue };
        let Some((_, literal)) = shape.trim().split_once("=>") else { continue };
        // What is left of the value is nothing: it was a string.
        if !literal.trim().trim_end_matches(\',\').trim().is_empty() {
            continue;
        }
        let Some((_, literal)) = line.trim().split_once("=>") else { continue };
' \
    $'    let body = function_lines(&lines, start);
    let mut names = Vec::new();
    for line in &body {
        let trimmed = line.trim();
        let Some((_, literal)) = trimmed.split_once("=>") else { continue };
        let literal = literal.trim().trim_end_matches(\',\').trim();
'
expect_catch "gate: the control methods read an arm out of a comment" \
    "cargo test -p xtask --profile ci a_method_in_a_comment_is_not_a_control_method"

# A brace inside a string is not a brace. The cut that removes a test item
# looked for the end of the item in the text, and a `}` inside a string ends
# a string, not an item. The tree's own `xtask/src/web.rs` has a raw string
# with a `}` at the start of a line: the cut ended there, and the eighteen
# test functions after it were read as production code. Measured, against the
# real gates on a copy of the tree, with the cut as it was before the fix and
# with the fix in it:
#
#   * a test that catches a panic, after the brace, was reported as an
#     undeclared panic boundary -- a false positive. The control, with the
#     same catch in production code, was reported by both.
#   * a limit whose every production use had been replaced by a literal, and
#     which a test after the brace mentioned, was reported as enforced -- a
#     false negative. The control, with nothing mentioning it, was reported
#     correctly by both.
#
# The end of an item is found by matching its braces in the code view, the
# way the end of a test-only method has been found since section 157. Section
# 160.
swap xtask/src/lists.rs \
    $'    let Some(close) = matching(code, open, b\'{\', b\'}\') else {
        return lines.len().saturating_sub(1);
    };
    line_of(code, close, lines)
' \
    $'    // The old rule: the first `}` in the first column ends the item.
    for (at, line) in lines.iter().enumerate().skip(index) {
        if *line == "}" {
            return at;
        }
    }
    lines.len().saturating_sub(1)
'
expect_catch "gate: the cut ends a test item at a brace inside a string" \
    "cargo test -p xtask --profile ci a_brace_in_a_string_does_not_end_a_test_module"

# Four gates still read the tree's source as text. A gate that reads a file
# as text reads its comments and its strings as if they were code, and a
# mention in a string is then a sentence about the code rather than the code
# itself. Measured, against the real gates on a copy of the tree, with the
# gates as they were before section 161 and with the fix in them:
#
#   * a crate root whose deny attribute stood only inside a string was
#     **accepted** -- a false negative. The control, with the attribute gone
#     altogether, was reported by both.
#   * a `mod` line that stood only inside a string was counted as a module,
#     and the gate that says the `mod` lines are sorted reported the tree's
#     own `xtask/src/lib.rs` as unsorted -- a false positive. The control,
#     with a real `mod` line out of order, was reported by both.
#   * a document's claim, satisfied by a mention inside a string, was
#     **accepted** -- a false negative. The control, with the claim false and
#     the name nowhere, was reported by both.
#   * a variant that stood only inside a block comment was counted as a
#     variant, so the gate that says how many areas there are reported the
#     tree's own "7 areas" as wrong -- a false positive. The control, with a
#     real variant added, was reported by both.
#
# Section 161.
swap xtask/src/lists.rs \
    '        let code = code_lines_kept(&body);' \
    '        let code = body.clone();'
expect_catch "gate: the crate root's deny attribute is read out of a string" \
    "cargo test -p xtask --profile ci a_deny_attribute_in_a_string_is_not_a_deny_attribute"

swap xtask/src/lists.rs \
    '        let Some(code_line) = code.get(number) else { continue };
        let code_line = code_line.trim();' \
    '        let code_line = source.lines().nth(number).unwrap_or_default().trim();'
expect_catch "gate: the module list reads a mod line out of a string" \
    "cargo test -p xtask --profile ci a_mod_line_in_a_string_is_not_a_module"

swap xtask/src/lists.rs \
    '                let view = code_lines_kept(&body);' \
    '                let view = body.clone();'
expect_catch "gate: a claim is satisfied by a mention inside a string" \
    "cargo test -p xtask --profile ci a_claim_satisfied_by_a_string_is_not_satisfied"

swap xtask/src/lists.rs \
    '    // The variants are **code**: a variant name inside a string is not a
    // variant, and a `}` inside a string does not end the enum. §161.
    let code = code_lines_kept(&text);' \
    '    // The variants are **code**: a variant name inside a string is not a
    // variant, and a `}` inside a string does not end the enum. §161.
    let code = text.clone();'
expect_catch "gate: the area count reads a variant out of a comment" \
    "cargo test -p xtask --profile ci a_variant_in_a_comment_is_not_a_variant"

# And a claim is about the file that follows it, not the file that came
# before. The gate that says every symbol a document claims is in a file
# paired a claim with the file on either side of it, so a claim and a file
# on the far side of a clause boundary were paired as if the document had
# said they belonged together. Measured: the tree's own sentence in
# `skills/craftkit-editing/SKILL.md` -- "the grammar is in `path.rs`; the
# listing is `PropGroup::flatten` in `property.rs`" -- is one claim, and the
# gate read it as two: it reported that `path.rs` does not have `flatten`
# when `path.rs` never claimed to. The control, with the claim naming one
# file, was reported once by both. Section 161.
swap xtask/src/lists.rs \
    '                let (code, file) = (left.1, right.1);' \
    '                let (code, file) = (right.1, left.1);'
expect_catch "gate: a claim is paired with the file before it" \
    "cargo test -p xtask --profile ci a_claim_after_a_clause_boundary_is_not_about_the_file_before_it"

# A name is not a piece of a longer name. Two gates ask whether a name is in
# a file, and both answered with a text test that a longer name satisfies:
# the claim gate with `code.contains(name)`, which `observe` satisfies for
# `serve`, and the symbol gate with `code.contains("fn " + name)`, which
# `fn the_ghost_test_two` satisfies for `the_ghost_test`. Measured, against
# the real gates on a copy of the tree, with the gates as they were before
# section 162 and with the fix in them:
#
#   * a document claiming that `Ghost::serve` is in `crates/core/src/ids.rs`
#     was **accepted** -- a false negative. `ids.rs` has `pub fn observe(..)`
#     and no `serve`. The control, with a name that is nowhere at all, was
#     reported by both.
#   * a document claiming that the test `the_ghost_test` is in
#     `crates/core/src/lib.rs` was **accepted** -- a false negative. The file
#     has `fn the_ghost_test_two()` and no `the_ghost_test`. The control, with
#     the test nowhere at all, was reported by both.
#
# Section 162.
swap xtask/src/lists.rs \
    '                    Claim::Method(name) => has_word(&view, name),' \
    '                    Claim::Method(name) => view.contains(name.as_str()),'
expect_catch "gate: a claim is satisfied by a name inside a longer name" \
    "cargo test -p xtask --profile ci a_claim_satisfied_by_a_substring_is_not_satisfied"

swap xtask/src/lists.rs \
    '        if has_word(&code, &format!("{keyword}{name}")) {' \
    '        if code.contains(&format!("{keyword}{name}")) {'
expect_catch "gate: a claimed test is satisfied by a longer test's name" \
    "cargo test -p xtask --profile ci a_test_satisfied_by_a_longer_name_is_not_satisfied"

# The matcher has to refuse an empty name, and it has to do so *quickly*:
# it searches with `find`, which answers at every position for an empty
# needle, and it advances its cursor to the end of the match, so an empty
# name would have left the cursor where it was and the search would never
# end. The mutation here is not "remove the guard" -- that would hang the
# run rather than fail it, and a mutation that hangs the suite is worse
# than no mutation -- but "call an empty name a word", which the test
# catches in a second. Section 162.
swap xtask/src/lists.rs \
    '    if name.is_empty() {
        return false;
    }' \
    '    if name.is_empty() {
        return true;
    }'
expect_catch "gate: the matcher calls an empty name a word" \
    "cargo test -p xtask --profile ci an_empty_name_is_not_a_word"

# A name is not a piece of a longer name, and four more gates asked the
# question with a text test. Measured, against the real gates on a copy of
# the tree, with the gates as they were before section 164 and with the fix
# in them. Each case has a control: the same tree with the thing really
# wrong, so a gate that said no to everything would pass the control too.
#
#   * a limit whose only use was inside `MAX_UNUSED_PER_NODE` was reported
#     as **enforced** -- a false negative. The control, with the limit
#     nowhere at all, was reported by both.
#   * a bound that only `MAX_DOCUMENT_BYTES_LIMIT` named in the prose was
#     reported as **named** -- a false negative. The control, with the bound
#     named nowhere, was reported by both.
#   * a budget that named the model's budget only inside a longer name was
#     reported as **derived from the model** -- a false negative. The
#     control, with the statement naming no model budget, was reported by
#     both.
#   * a file that named `craftkit_core::writer::write_json_pretty` was
#     reported as written with the tree's **writer** -- a false negative.
#     The control, with the file naming no writer at all, was reported by
#     both.
#
# Section 164.
swap xtask/src/lists.rs \
    '            if has_word(&body, name) && !used.contains(name) {' \
    '            if body.contains(name.as_str()) && !used.contains(name) {'
expect_catch "gate: the limits gate reads a limit inside a longer name" \
    "cargo test -p xtask --profile ci a_limit_only_a_longer_name_uses_is_still_caught"

swap xtask/src/lists.rs \
    '            if has_word(&prose, &name) {' \
    '            if prose.contains(&name) {'
expect_catch "gate: the prose names a bound inside a longer name" \
    "cargo test -p xtask --profile ci a_bound_only_a_longer_name_names_in_the_prose_is_not_named"

swap xtask/src/lists.rs \
    '            if has_word(&statement, name) {' \
    '            if statement.contains(name) {'
expect_catch "gate: a budget names the model's budget inside a longer name" \
    "cargo test -p xtask --profile ci a_budget_that_names_only_a_longer_name_is_not_derived_from_the_model"

swap xtask/src/lists.rs \
    '        if !has_word(&code, "craftkit_core::writer::write_json") {' \
    '        if !code.contains("craftkit_core::writer::write_json") {'
expect_catch "gate: the writer gate reads a path inside a longer path" \
    "cargo test -p xtask --profile ci the_writer_is_not_a_path_that_starts_the_same_way"

# The gate that says every field the model's types have is hashed read the
# **shape** of the hash out of the tree's source, and it read it as **text**:
# an impl block, a hash method and a struct declaration were found the way a
# person reads a file. A string that mentions one of those is a sentence
# about one, not one. Measured, against the real gate on a copy of the tree,
# with the gate as it was before section 165 and with the fix in it. Each
# case has a control: the same tree with the thing really wrong, so a gate
# that said no to everything would pass the control too.
#
#   * an `impl Ghost {` inside a raw string in `crates/model/src/fingerprint.rs`
#     was reported as a hash for a type the tree does not have -- a false
#     positive. The control, the same string with no impl in it, was green
#     in both.
#   * a `}` inside a raw string ended the impl block, so the hash method
#     after it was not a target at all and the type's fields stopped being
#     checked -- a false negative: the hash had stopped reading `Node`'s
#     field `name`, and the gate said nothing about it. The control, with
#     the hash really not reading the field, was reported by both.
#   * a `pub struct Node` inside a raw string in `crates/model/src/document.rs`
#     was the type the tree has, so the gate asked the hash for the string's
#     field `ghost` -- a field the model does not have -- and did not ask for
#     `name`, which it has: a false positive and a false negative in one
#     gate. The control, with the model really having the field, was
#     reported by both.
#
# Section 165.
swap xtask/src/lists.rs \
    '    // The **shape** comes from the code: an impl block or a hash method
    // inside a string is a sentence about one, not one. §165.
    let code = code_lines_kept(&source);' \
    '    // The **shape** comes from the code: an impl block or a hash method
    // inside a string is a sentence about one, not one. §165.
    let code = source.clone();'
expect_catch "gate: an impl inside a string is not an impl block" \
    "cargo test -p xtask --profile ci an_impl_inside_a_string_is_not_an_impl_block"

swap xtask/src/lists.rs \
    '    // The **shape** comes from the code: an impl block or a hash method
    // inside a string is a sentence about one, not one. §165.
    let code = code_lines_kept(&source);' \
    '    // The **shape** comes from the code: an impl block or a hash method
    // inside a string is a sentence about one, not one. §165.
    let code = source.clone();'
expect_catch "gate: a brace inside a string does not end an impl block" \
    "cargo test -p xtask --profile ci a_brace_inside_a_string_does_not_end_an_impl_block"

swap xtask/src/lists.rs \
    '        // The code, not the text: a `struct` inside a string is a sentence
        // about a struct, and a `}` inside one does not end it. §165.
        let code = code_lines_kept(&text);' \
    '        // The code, not the text: a `struct` inside a string is a sentence
        // about a struct, and a `}` inside one does not end it. §165.
        let code = text.clone();'
expect_catch "gate: a struct inside a string is not the struct" \
    "cargo test -p xtask --profile ci a_struct_inside_a_string_is_not_the_struct"

# Two more gates read a **shape** out of the tree's source as text, and
# section 165's audit asked for exactly this: not "is the subject code?" but
# "is the shape read as code?". Measured, against the real gates on a copy of
# the tree, with the gates as they were before section 166 and with the fix
# in them. Each case has a control: the same tree with the thing really
# wrong, so a gate that said no to everything would pass the control too.
#
#   * the session gate found its `impl Session` block in the **text** and
#     ended it at the first `}` in the first column. A `}` inside a raw
#     string ended the block early, so a public method after it -- a public
#     way into the design, which is the thing the gate exists to stop -- was
#     not reported: a false negative. The control, with the block intact, was
#     reported by both.
#   * the same gate looked for the header in the text too, so an
#     `impl Session {` inside a raw string was the block it read, and that
#     block ended at the string's own `}` -- before the real one, so the real
#     methods went unchecked: a false negative. The control, with the string
#     out of the way, was reported by both.
#   * the assets gate found the embed calls in the **text** and cut each line
#     at the first `//`, so a **block** comment that showed a call was
#     reported as an embed of a file the tree does not have: a false
#     positive. The control, with the call really in the code, was reported
#     by both.
#   * the same gate, and a **raw string**: a raw string holds a call as
#     plainly as a comment does, and it was reported as an embed too: a
#     false positive. The control was reported by both.
#
# Section 166.
swap xtask/src/lists.rs \
    '    let code = code_lines_kept(source);
    let start = code.find(header)?;
    let brace = header.rfind('"'"'{'"'"')?;
    let close = matching(&code, start + brace, b'"'"'{'"'"', b'"'"'}'"'"')?;
    let block = &source[start + header.len()..close];' \
    '    let start = source.find(header)?;
    let body = &source[start + header.len()..];
    let end = body.find("\n}\n")?;
    let block = &body[..end];'
expect_catch "gate: a brace inside a string ends the session's block" \
    "cargo test -p xtask --profile ci a_brace_inside_a_string_does_not_end_the_session_block"

swap xtask/src/lists.rs \
    '    let code = code_lines_kept(source);
    let start = code.find(header)?;
    let brace = header.rfind('"'"'{'"'"')?;
    let close = matching(&code, start + brace, b'"'"'{'"'"', b'"'"'}'"'"')?;
    let block = &source[start + header.len()..close];' \
    '    let start = source.find(header)?;
    let body = &source[start + header.len()..];
    let end = body.find("\n}\n")?;
    let block = &body[..end];'
expect_catch "gate: an impl inside a string is not the session's block" \
    "cargo test -p xtask --profile ci an_impl_inside_a_string_is_not_the_session_block"

swap xtask/src/assets.rs \
    '        let code = crate::lists::code_lines_kept(&text);
        for (number, (code_line, text_line)) in code.lines().zip(text.lines()).enumerate() {
            for call in EMBED_CALLS {
                let Some(at) = code_line.find(call) else { continue };
                let rest = &text_line[at + call.len()..];' \
    '        for (number, line) in text.lines().enumerate() {
            let code_line = line.split("//").next().unwrap_or(line);
            for call in EMBED_CALLS {
                let Some(at) = code_line.find(call) else { continue };
                let rest = &code_line[at + call.len()..];'
expect_catch "gate: an embed call in a block comment is not an embed" \
    "cargo test -p xtask --profile ci an_embed_call_in_a_block_comment_is_not_an_embed"

swap xtask/src/assets.rs \
    '        let code = crate::lists::code_lines_kept(&text);
        for (number, (code_line, text_line)) in code.lines().zip(text.lines()).enumerate() {
            for call in EMBED_CALLS {
                let Some(at) = code_line.find(call) else { continue };
                let rest = &text_line[at + call.len()..];' \
    '        for (number, line) in text.lines().enumerate() {
            let code_line = line.split("//").next().unwrap_or(line);
            for call in EMBED_CALLS {
                let Some(at) = code_line.find(call) else { continue };
                let rest = &code_line[at + call.len()..];'
expect_catch "gate: an embed call in a string is not an embed" \
    "cargo test -p xtask --profile ci an_embed_call_in_a_string_is_not_an_embed"

# The cut that removes a file's test items did not know that a file under a
# `tests` directory is a test file from its first line to its last: Rust
# puts a crate's integration tests there, and each file in it is a crate of
# its own. Measured, against the real gates on a copy of the tree, with the
# gates as they were before section 167 and with the fix in them. Each case
# has a control: the same tree with the thing really wrong, so a gate that
# said no to everything would pass the control too.
#
#   * a `catch_unwind` in a file under `tests/` was reported as a boundary
#     the pattern document does not declare -- a false positive: a test that
#     catches a panic to prove it was caught is not a boundary. The control,
#     with the catch in a production file, was reported by both.
#   * a limit whose only use was in a file under `tests/` was reported as
#     **enforced** -- a false negative: nothing in the product enforced it.
#     The control, with the production use replaced by a literal and no test
#     file, was reported by both.
#
# Section 167.
swap xtask/src/lists.rs \
    '    path.components().any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))' \
    '    path.components().any(|part| part.as_os_str() == std::ffi::OsStr::new("test"))'
expect_catch "gate: a file under a tests directory is a test file" \
    "cargo test -p xtask --profile ci a_file_under_a_tests_directory_is_a_test_file"

swap xtask/src/lists.rs \
    '    path.components().any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))' \
    '    path.components().any(|part| part.as_os_str() == std::ffi::OsStr::new("test"))'
expect_catch "gate: a test file does not catch a panic as a boundary" \
    "cargo test -p xtask --profile ci a_test_file_does_not_catch_a_panic_as_a_boundary"

# ------------------------------------------------- the dependency policy ----
# Section 173. The licence policy was a gate with no name in the tree, and it
# had never been green: `cargo deny check` failed three of its four sections.
# It is a named gate now, and the three cases below break the three links that
# make it a gate rather than a line of YAML.
#
# The first two are the rule in `xtask/src/deny.rs`, which is a function of its
# own so that a test can pin it: a gate that could not run, and a gate whose
# policy does not hold, must both be something other than a pass. Neither
# needs cargo-deny installed, which is why they are unit tests and not runs of
# the tool.
swap xtask/src/deny.rs \
    '        return if required { Conclusion::Required } else { Conclusion::NotChecked };' \
    '        return Conclusion::Pass;'
expect_catch "policy: a tool that is not installed counts as a pass" \
    "cargo test -p xtask --profile ci a_tool_that_is_not_installed_is_not_a_pass"

# The variable is renamed so the swap still compiles: a mutation that does not
# compile tests nothing, and the script says so loudly.
swap xtask/src/deny.rs \
    '        Some(code) => Conclusion::Failed(code),' \
    '        Some(_code) => Conclusion::Pass,'
expect_catch "policy: a policy that does not hold counts as a pass" \
    "cargo test -p xtask --profile ci a_policy_that_does_not_hold_is_a_failure_and_not_a_skip"

# The third link: a command `ci` does not run must be run by name in the
# workflow. The workflow's prose still mentions `cargo xtask deny` after this
# swap -- in a comment -- so the case is also the one that says a mention is
# not a command. The first version of the check searched the whole file and
# passed on exactly this tree.
swap .github/workflows/ci.yml \
    '        run: cargo xtask deny' \
    '        run: cargo deny check'
expect_catch "policy: the workflow stops running the tree's gate by name" \
    "cargo xtask lists"

# And the policy itself: the licence the tree's own dependencies need, taken
# out of the allow list. This one needs the tool, so it is skipped where the
# tool is not installed -- reported as skipped, never as caught.
swap deny.toml \
    '    "BSL-1.0",
' \
    ''
expect_catch "policy: a licence the tree needs is taken out of the allow list" \
    "cargo xtask deny" \
    "no" \
    "cargo-deny"

# ------------------------------------------------------- the version flag ----
# Section 174. Two of the three programs in the tree printed their version and
# the headless CLI did not -- and the headless CLI is the one an agent
# drives, so it is the one whose version matters most.
#
# The version line stops carrying the version. It is the whole of the rule:
# the number comes from the manifest, at compile time, and a version written
# out by hand would be wrong after the next release with nothing to say so.
swap apps/craftkit-cli/src/cli.rs \
    '    format!("craftkit-cli {}", env!("CARGO_PKG_VERSION"))' \
    '    format!("craftkit-cli")'
expect_catch "cli: the version line stops carrying the version" \
    "cargo test -p craftkit-cli --profile ci the_version_line_names_the_program_and_its_version"

# And the parser stops accepting the long spelling of the flag.
swap apps/craftkit-cli/src/args.rs \
    '        "-V" | "--version" => Ok(Some(Command::Version)),' \
    '        "-V" => Ok(Some(Command::Version)),'
expect_catch "cli: the parser stops accepting the version flag" \
    "cargo test -p craftkit-cli --profile ci the_version_is_asked_for_in_two_ways"

# --------------------------------------------------- the property tests ----
# Section 175. The house's promise is that hostile input is refused, and it
# was checked by a table of inputs somebody thought of. The first two breaks
# are the ones the table misses: each is a bound off by one, in a place where
# every hand-written test used a value far enough from the edge that the slip
# changed nothing. The third is about the suites themselves.
#
# The gates named here are the **properties**, and they are named one by one
# because that is the point: the unit tests do not catch the first two, so a
# case that ran the whole crate would not say that the property is what
# caught it.
swap crates/core/src/readers.rs \
    '                    content_len = content_len.saturating_add(index);' \
    '                    content_len = content_len.saturating_add(index + 1);'
expect_catch "reader: the line's length counts the newline" \
    "cargo test -p craftkit-core --profile ci a_line_longer_than_the_limit_is_refused_with_its_true_length"

swap crates/core/src/params.rs \
    '                if s.len() > max {' \
    '                if s.len() > max + 1 {'
expect_catch "params: the text cap is one too large" \
    "cargo test -p craftkit-core --profile ci a_text_parameter_never_exceeds_its_limit"

# And the suites stop running from a fixed seed, so a CI run can fail at
# random. The rule is a test rather than a comment, so it can be caught.
swap crates/core/tests/properties.rs \
    '    Config { failure_persistence: None, rng_seed: RngSeed::Fixed(SEED), cases: CASES, ..Config::default() }' \
    '    Config { failure_persistence: None, rng_seed: RngSeed::Random, cases: CASES, ..Config::default() }'
expect_catch "property: the suites stop running from a fixed seed" \
    "cargo test -p craftkit-core --profile ci the_property_suites_run_from_a_fixed_seed_and_write_nothing_down"

# ------------------------------------------------- the MCP resources ----
# Section 176. The server served tools and nothing else, so a client that
# read the protocol's other half was told the method did not exist. The
# rules the port carries are the ones that keep a surface honest: what is
# advertised can be read, what is read is advertised, the capability says
# so, a missing resource says so, and a resource holds what its tool holds.
#
# The gates named here are the **tests**, one by one, because that is the
# point: a case that ran the whole crate would not say which rule caught it.

# The first takes a resource out of the read dispatch while the list still
# advertises it: the defect the family had with tools.
swap crates/mcp/src/resources.rs \
    '    if let Some(resource) = RESOURCES.iter().find(|r| r.uri == uri) {' \
    '    if let Some(resource) = RESOURCES.iter().find(|r| r.uri == uri && !r.uri.ends_with("commands")) {'
expect_catch "mcp: a resource is advertised and cannot be read" \
    "cargo test -p craftkit-mcp --profile ci every_advertised_resource_can_be_read"

# The second advertises a capability the server does not have, and hides the
# one it does.
swap crates/mcp/src/rpc.rs \
    '        caps.insert("resources".to_string(), json!({ "subscribe": false, "listChanged": false }));' \
    '        caps.insert("prompts".to_string(), json!({ "subscribe": false, "listChanged": false }));'
expect_catch "mcp: the handshake hides the capability the server has" \
    "cargo test -p craftkit-mcp --profile ci initialize_advertises_the_resources_it_serves"

# The third answers a missing resource with "no such method", which is how a
# client concludes the server has no resources at all.
swap crates/mcp/src/server.rs \
    '            code::RESOURCE_NOT_FOUND,' \
    '            code::METHOD_NOT_FOUND,'
expect_catch "mcp: a missing resource says it is a missing method" \
    "cargo test -p craftkit-mcp --profile ci an_unknown_resource_is_refused_with_its_own_code"

# And the fourth lets a resource answer with something other than what its
# tool holds: two answers to one question.
swap crates/mcp/src/resources.rs \
    '    Ok(craftkit_engine::catalogue())' \
    '    Ok(Json::Array(Vec::new()))'
expect_catch "mcp: a resource holds something other than its tool" \
    "cargo test -p craftkit-mcp --profile ci a_resource_holds_what_its_tool_holds"

# ------------------------------------------------- the hostile inputs ----
# Section 177. The tree's promise is that hostile input is refused. It was
# checked by a table of inputs somebody thought of, and the mutation suite
# mutates the *tree*, not the *input* — so the input side was checked by
# nobody. The properties here fire *generated* values at every parameter of
# every command, and look at the answer rather than at the absence of a
# crash.
#
# The two breaks below are `doc.range`'s own bounds, one off. Both were
# caught by the property and by **nothing** in the tree's own tests: with
# the bound loosened the command succeeded, the document it left was one
# the file format refuses, and the engine reported the tree's own
# unexpected failure instead of a refusal with a reason.
#
# The gate named here is the **property**, because that is the point: a case
# that ran the whole crate would not say that the property is what caught
# it.

# The first lets the frame rate be zero. The model refuses a rate of zero,
# so the command leaves a document the file reader would refuse — and the
# engine's own check turns that into "failed unexpectedly".
swap crates/engine/src/commands.rs \
    'if f < 1 || f > i64::from(limits::MAX_FRAME_RATE) {' \
    'if f < 0 || f > i64::from(limits::MAX_FRAME_RATE) {'
expect_catch "engine: the frame rate may be zero" \
    "cargo test -p craftkit-engine --profile ci no_command_reports_an_internal_error"

# The second lets the range be empty: the end at the start.
swap crates/engine/src/commands.rs \
    'if end <= start {' \
    'if end < start {'
expect_catch "engine: the range may be empty" \
    "cargo test -p craftkit-engine --profile ci no_command_reports_an_internal_error"

# ------------------------------------------------------- the MCP prompts ----
# Section 178. The server had tools and resources and no prompts, so the
# protocol's third part was a method that did not exist. The rules the port
# carries are the ones that keep an instruction honest: it names a command
# the tree has, the message is built from the registry's own words, what is
# advertised can be fetched, and the suggestions are the tree's and not a
# copy of them.
#
# The first break is the one that matters most: a prompt that names a
# command the registry does not have is an instruction the agent will follow
# into a call that is not there, and nothing in the tree would have said so.

swap crates/mcp/src/prompts.rs \
    'command: "prop.set",' \
    'command: "prop.setx",'
expect_catch "mcp: a prompt names a command the registry does not have" \
    "cargo test -p craftkit-mcp --profile ci every_prompt_names_a_command_the_registry_has"

# The second leaves the placeholder in the message, so the prompt tells the
# agent to call a command whose name was never filled in.
swap crates/mcp/src/prompts.rs \
    '    text = text.replace("{command}", &id);' \
    '    text = text.clone();'
expect_catch "mcp: the message does not carry the registry's words" \
    "cargo test -p craftkit-mcp --profile ci the_message_carries_the_registry_s_words"

# The third advertises a prompt the dispatch cannot fetch — the defect the
# family had with its tools.
swap crates/mcp/src/prompts.rs \
    '.find(|p| p.name == name)' \
    '.filter(|p| p.name != "add-node").find(|p| p.name == name)'
expect_catch "mcp: a prompt is advertised and cannot be fetched" \
    "cargo test -p craftkit-mcp --profile ci every_advertised_prompt_can_be_fetched"

# And the fourth answers with a copy of the tree's values, which goes stale
# the day a node is renamed.
swap crates/mcp/src/prompts.rs \
    '        Live::NodeKinds => craftkit_model::NodeKind::all().iter().map(|k| k.label().to_string()).collect(),' \
    '        Live::NodeKinds => vec!["filter".to_string()],'
expect_catch "mcp: the suggestions are a copy and not live" \
    "cargo test -p craftkit-mcp --profile ci the_suggestions_are_live"

# --------------------------------------------- the protocol's version ----
# Section 179. The server spoke one revision of the protocol and answered
# every client with it, whatever the client asked for, and the protocol's
# fourth method — the one that suggests values for an argument — was a method
# that did not exist.
#
# The rules the port carries are the specification's own: the handshake
# answers the client's revision when this server speaks it and one it speaks
# when it does not; a capability is advertised only to a client that
# negotiated a revision which has it; and the suggestions are filtered by
# what the caller has typed.

# The first answers a revision this server does not speak — the client's,
# echoed. The specification forbids it: the server must answer a revision it
# supports.
swap crates/mcp/src/lib.rs \
    '        Some(version) if SUPPORTED_VERSIONS.contains(&version) => version,' \
    '        Some(version) if !version.is_empty() => version,'
expect_catch "mcp: the server answers a revision it does not speak" \
    "cargo test -p craftkit-mcp --profile ci the_handshake_answers_a_revision_this_server_speaks"

# The second answers the server's own revision even when the client's is one
# it speaks — so a client that asked for a revision and got another may
# disconnect, which the specification tells it to do.
swap crates/mcp/src/lib.rs \
    '        Some(version) if SUPPORTED_VERSIONS.contains(&version) => version,' \
    '        Some(version) if SUPPORTED_VERSIONS.contains(&version) => PROTOCOL_VERSION,'
expect_catch "mcp: the server does not answer the revision the client asked for" \
    "cargo test -p craftkit-mcp --profile ci the_handshake_answers_a_revision_this_server_speaks"

# The third offers a capability to a client whose revision has never heard of
# it: `completions` arrived in 2025-03-26.
swap crates/mcp/src/rpc.rs \
    '    version >= "2025-03-26"' \
    '    version >= "2024-11-05"'
expect_catch "mcp: a 2024 client is offered completions" \
    "cargo test -p craftkit-mcp --profile ci a_capability_is_advertised_only_to_a_client_that_can_use_it"

# And the fourth answers with every value, every time — which is not a
# suggestion.
swap crates/mcp/src/rpc.rs \
    '    value.to_ascii_lowercase().starts_with(&typed.to_ascii_lowercase())' \
    '    let _ = value;
    true'
expect_catch "mcp: the suggestions are not filtered by what was typed" \
    "cargo test -p craftkit-mcp --profile ci the_suggestions_are_filtered_by_what_was_typed"

# --------------------------------------------------------- the fuzzer ----
# Section 180. The reader was the widest door in the program and nothing was
# pushing on it: every file the tree ever read was written by the tree's own
# writer.
#
# The rules the port carries: a crash is three things (a signal, the panic
# exit code, the panic message), a hang is a hang and not a crash, the
# corpus is written by the tree's own writer, the seed decides the run, a
# smaller crash is still a crash, and — the rule the whole thing rests on —
# the fuzzer can find a crash at all.

# The first: the panic message is not a crash. This is the one that matters,
# because the engine runs every command under `catch_unwind`: a panic inside
# a command leaves the process exiting 1, and only the message says what
# happened.
swap xtask/src/fuzz.rs \
    '    if stdout.contains(PANIC_MARKER) || stderr.contains(PANIC_MARKER) {' \
    '    if false {'
expect_catch "fuzz: a panic the program survived is not a crash" \
    "cargo test -p xtask --profile ci a_panic_message_is_a_crash_even_when_the_process_survived"

# The second: the panic exit code is not a crash.
swap xtask/src/fuzz.rs \
    '    if code == Some(PANIC_EXIT_CODE) {' \
    '    if code == Some(PANIC_EXIT_CODE - 1) {'
expect_catch "fuzz: the panic exit code is not a crash" \
    "cargo test -p xtask --profile ci a_panic_exit_code_is_a_crash"

# The third: a hang is reported as a crash — the wrong kind of bug, and a
# report that sends the reader to the wrong place.
swap xtask/src/fuzz.rs \
    '    if timed_out {' \
    '    if false {'
expect_catch "fuzz: a hang is reported as a crash" \
    "cargo test -p xtask --profile ci a_hang_is_a_hang_and_not_a_crash"

# The fourth: the corpus is written by something that is not the tree's
# writer. The seed is then a file the reader turns away — so every mutant of
# it is refused at the first line, the fuzzer tests nothing, and it is green
# for the wrong reason. The corpus is the tree's *program* writing it: the
# CLI's `sample --out`.
swap apps/craftkit-cli/src/cli.rs \
    '    craftkit_io::save(out, session.document()).map_err(|e| e.to_string())?;' \
    '    std::fs::write(out, "{}").map_err(|e| e.to_string())?;'
expect_catch "fuzz: the corpus is not a design the tree reads" \
    "cargo test -p craftkit-cli --profile ci the_sample_design_is_written_and_read_back_by_the_tree"

# The fifth: the seed is ignored. The run is then the same twenty inputs,
# whatever the seed says — reproducible, and not a search.
swap xtask/src/fuzz.rs \
    '        Rng(seed)' \
    '        Rng(0)'
expect_catch "fuzz: the seed does not decide the run" \
    "cargo test -p xtask --profile ci the_seed_decides_the_run"

# The sixth: the minimiser throws away the smaller input — or rather, keeps
# the smaller one that does *not* break, which is worse than keeping
# nothing.
swap xtask/src/fuzz.rs \
    '            best.truncate(cut);' \
    '            best.truncate(best.len());'
expect_catch "fuzz: the minimiser keeps the whole file" \
    "cargo test -p xtask --profile ci the_minimiser_keeps_a_smaller_input_that_still_breaks"

# And the seventh, which is the rule the whole thing rests on: the fake
# child does not die. Without it the fuzzer finds nothing, finds nothing
# always, and looks exactly like a fuzzer that works.
swap xtask/src/fuzz.rs \
    '        panic!("the fake fuzz child, on purpose");' \
    '        let _ = "the fake fuzz child, on purpose";'
expect_catch "fuzz: the fuzzer finds nothing, because the child does not die" \
    "cargo test -p xtask --profile ci the_fuzzer_finds_a_crash_it_made"

# ------------------------------------------------------------ the step bound ----
# Section 181. The tree had two doors that run a list of commands -- a script
# and a crash replay -- and two numbers for them. The replay door's number was
# 20 000, which was more than twice what a report this tree wrote can hold and
# twice what the model allows; the script door had no number at all, only the
# file's bytes.
#
# The rules the port carries: each door refuses a list that is too long, both
# doors read the *same* number, and the tree declares that number once.

# The first: the script door stops refusing. Without it a script of the
# limit's size holds 29 958 short steps, and every one of them clones the
# document.
swap apps/craftkit-cli/src/cli.rs \
    '    if script.commands.len() > craftkit_engine::MAX_COMMAND_STEPS {' \
    '    if script.commands.len() > craftkit_engine::MAX_COMMAND_STEPS * 2 {'
expect_catch "the script door does not refuse a script with too many steps" \
    "cargo test -p craftkit-cli --profile ci a_script_with_too_many_steps_is_refused_with_the_bound_in_the_message"

# The second: the replay door stops refusing.
swap crates/engine/src/crash.rs \
    '    if report.replay.len() > crate::MAX_COMMAND_STEPS {' \
    '    if report.replay.len() > crate::MAX_COMMAND_STEPS * 2 {'
expect_catch "the replay door does not refuse a report with too many steps" \
    "cargo test -p craftkit-engine --profile ci a_replay_with_too_many_steps_is_refused_before_it_runs"

# The third: the script door grows a **copy** of the number. The number is
# still right -- it is the same number -- so every test that checks the value
# passes. What is broken is the *sharing*: from today the two doors can drift,
# and the gate that says they share one bound is the only thing that sees it.
swap apps/craftkit-cli/src/cli.rs \
    '    if script.commands.len() > craftkit_engine::MAX_COMMAND_STEPS {
        return Err(format!(
            "the script has {} step(s), and a script runs at most {} step(s) \
             (`craftkit_engine::MAX_COMMAND_STEPS`): \
             split it into scripts of at most that many steps",
            script.commands.len(),
            craftkit_engine::MAX_COMMAND_STEPS
        ));
    }' \
    '    if script.commands.len() > 10_000 {
        return Err(format!(
            "the script has {} step(s), and a script runs at most {} step(s) \
             (the bound): \
             split it into scripts of at most that many steps",
            script.commands.len(),
            10_000
        ));
    }'
expect_catch "gate: the script door keeps a copy of the bound instead of the tree's" \
    "cargo test -p xtask --profile ci the_two_doors_share_one_step_bound"

# And the fourth: the bound is declared a second time. One bound, one name,
# one number -- a second declaration is a second number, even when the two
# agree today.
swap crates/engine/src/crash.rs \
    'pub use crate::session::TraceStep as ReplayStep;' \
    'pub use crate::session::TraceStep as ReplayStep;

/// A second home for the bound.
pub const MAX_COMMAND_STEPS: usize = 10_000;'
expect_catch "gate: the tree declares the step bound twice" \
    "cargo test -p xtask --profile ci the_two_doors_share_one_step_bound"

# ------------------------------------------------------- the script's writer ----
# Section 182. The tree read three files and could write only one of them: the
# design file had a writer (craftkit-cli sample --out), the script had none,
# and the crash report had only the desktop app's panic hook.
#
# Without a writer for the script, the fuzzer's rule -- the corpus is written
# by the tree's own program -- could not be applied to the script door at
# all, and a hand-written corpus would have been a second copy of the format.
#
# The rules the port carries: the tree writes a script, the script it writes
# is one the tree reads, it is the sample's own commands (one copy), and the
# fuzzer breaks that shape rather than a design's.

# The first: the tree's script writer writes a script the tree refuses. A
# corpus the reader turns away tests nothing.
swap apps/craftkit-cli/src/cli.rs \
    '    let text = script_text(&script_value("the sample design", None, commands))?;' \
    '    let text = script_text(&json!({ "name": "the sample design", "commands": "not an array" }))?;'
expect_catch "the tree's script writer writes a script the tree refuses" \
    "cargo test -p craftkit-cli --profile ci the_script_is_written_and_read_back_by_the_tree"

# The second: the script writer keeps a copy of the sample's commands instead
# of the sample's own. The script would still run -- so the first test passes
# -- and it would drift from the sample the moment the sample changed.
swap apps/craftkit-cli/src/cli.rs \
    '    let commands: Vec<Json> = craftkit_engine::sample_steps()
        .into_iter()
        .map(|(command, params)| json!({ "command": command, "params": params }))
        .collect();' \
    '    let commands: Vec<Json> =
        vec![json!({ "command": "node.add", "params": { "name": "In", "kind": "source" } })];'
expect_catch "the script writer keeps a copy of the sample's commands" \
    "cargo test -p craftkit-cli --profile ci the_script_is_the_sample_designs_own_commands"

# The third: the fuzzer's corpus for the script door is written by something
# that is not the tree's writer. The seed is then a file the reader turns
# away, the seed check refuses it, and the run stops -- which is the rule
# working, but only if the rule is there.
swap xtask/src/fuzz.rs \
    '    let args = vec!["script".to_string(), "--out".to_string(), path.to_string_lossy().into_owned()];' \
    '    let args = vec!["sample".to_string(), "--out".to_string(), path.to_string_lossy().into_owned()];'
expect_catch "the script door's corpus is not written by the tree's writer" \
    "cargo xtask fuzz --target script --iters 5 --out /tmp/craftkit-mutation-fuzz"

# The fourth: the script's shape is not broken at all. The other six
# mutations break the file, and the reader turns it away at the first line --
# so a shape mutation that changes nothing is a fuzzer that only ever tests
# the JSON parser.
swap xtask/src/fuzz.rs \
    'fn break_the_script(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {' \
    'fn break_the_script(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    let _ = rng;
    return bytes.to_vec();'
expect_catch "the script's shape is not broken" \
    "cargo test -p xtask --profile ci the_script_s_shape_is_broken_with_the_file_still_valid_json"

# The fifth: the script door's child is the wrong command. A design is read
# by `eval` and a script by `run`, and handing one to the other refuses it
# at the first line.
swap xtask/src/fuzz.rs \
    '            Target::Script => vec!["run".to_string()],' \
    '            Target::Script => vec!["eval".to_string()],'
expect_catch "the script door's child reads the wrong kind of file" \
    "cargo test -p xtask --profile ci each_target_s_child_reads_that_kind_of_file"

# And the sixth: a run covers one door when it should cover them all. A
# fuzzer that watches one door is watching the door the tree already knew
# about.
swap xtask/src/fuzz.rs \
    '            Selection::All => Target::ALL.to_vec(),' \
    '            Selection::All => vec![Target::Design],'
expect_catch "a run covers one door when it should cover them all" \
    "cargo test -p xtask --profile ci a_run_covers_every_target_unless_one_is_named"

# ---------------------------------------------------- the crash report's door ----
# Section 183. The tree has three readers, and two of them kept the promise
# "if a file was read, what it holds must be valid": the design file's and
# the script's base's. The crash report's reader kept it for the report's
# BASE and not for its DESIGN -- and the design is the report's target: what
# the replay has to end up with.
#
# So a report whose design the model refuses was read, shown and replayed,
# while the same document in a design file was refused.

# The first: the reader stops checking the design.
swap crates/engine/src/crash.rs \
    '    report.design = checked_document(report.design, "the report'"'"'s design")?;' \
    '    let _ = &report.design;'
expect_catch "the crash reader stops checking the report's design" \
    "cargo test -p craftkit-engine --profile ci a_report_whose_design_the_model_refuses_is_refused"

# The third: the reason does not name the document that is wrong. A report has
# two documents, and the reason has to say which one is broken.
swap crates/engine/src/crash.rs \
    '    doc.validate().map_err(|e| format!("{which} is not usable: {e}"))?;' \
    '    doc.validate().map_err(|e| format!("a document is not usable: {e}"))?;'
expect_catch "the reason does not name the document that is wrong" \
    "cargo test -p craftkit-engine --profile ci a_report_whose_design_the_model_refuses_is_refused"

# ------------------------------------------------- the script's own anchors ----
# Section 184. The instrument's anchors are text in the tree. The `swap`
# helper refuses an anchor that is not there, and it refuses it when the case
# is reached -- up to twenty-five minutes into a run, with the tree already
# changed twenty times -- and the report says what the anchor was and never
# where it had been.
#
# The gate is `cargo xtask anchors`, and it is a command of its own rather
# than a phase of `lists`. A mutation removes the anchor it names -- that is
# what a mutation is -- so an anchor gate inside `lists` would report every
# case as caught, whatever the case was for, and the eleven cases that run
# `lists` would say nothing about the gate they were written for.
#
# The first break: an anchor that moved. The reader's own limit changes by
# one, so the text the case names is not in the file any more. The swap
# changes nothing the case was about; it is here to move the anchor.
swap crates/core/src/readers.rs \
    '    reader.take(max as u64 + 1).read_to_string(&mut out).map_err(io_error)?;' \
    '    reader.take(max as u64 + 2).read_to_string(&mut out).map_err(io_error)?;'
expect_catch "anchors: an anchor that moved is not found" \
    "cargo xtask anchors"

# The second break: an anchor that names two places. `swap` replaces the
# first, so an anchor that matches twice is a case about whichever of the two
# happens to be written first -- and the tree grows a second copy of a line
# every time somebody pastes a command spec.
swap crates/engine/src/crash.rs \
    'pub use crate::session::TraceStep as ReplayStep;' \
    'pub use crate::session::TraceStep as ReplayStep;
pub use crate::session::TraceStep as ReplayStep;'
expect_catch "anchors: an anchor that names two places is reported" \
    "cargo xtask anchors"

# ------------------------------------------------------ the crash report's folder ----
# Section 185. The crash report was a folder of three files: the report, a copy
# of the design and a copy of the steps. Nothing read the design's copy, and
# the replay's copy was read by a door nothing called -- so a report that
# arrived with the copies disagreeing had two answers, and the tree noticed
# neither. Measured: `replay_in` said the report held one command and
# `show_in` said it held nine.
#
# The first break: the replay door reads the second copy again. The tree has
# one copy of the steps now, and the two doors have to agree about it.
swap crates/engine/src/crash.rs \
    '    Ok(show_in(dir, id)?.replay)' \
    '    let path = dir.join(sanitize(id)).join("replay.json");
    let body = read_bounded(&path, MAX_REPORT_BYTES)?;
    serde_json::from_str(&body).map_err(|e| format!("{} is not a readable replay: {e}", path.display()))'
expect_catch "crash: the replay door reads a second copy of the steps" \
    "cargo test -p craftkit-engine --profile ci every_door_agrees"

# The second break: the report's folder is created instead of the report
# being filed in it. The folder is then there and the report is not, and the
# crash report is the only evidence a crash happened.
swap crates/engine/src/crash.rs \
    '    std::fs::rename(&staging, &dir).map_err(|e| {' \
    '    std::fs::create_dir_all(&dir).map_err(|e| {'
expect_catch "crash: the report's folder is created instead of the report being filed in it" \
    "cargo test -p craftkit-engine --profile ci recording_a_crash_writes_one_file"

# ------------------------------------------------ the properties' corpora ----
# Section 186. The crash report's properties wrote to one file, and they run
# at the same time: one could read another's corpus, or a half-written one,
# and then it asserted nothing -- the corpus it was supposed to break was
# not the one it read. Measured: with six properties on that file, the sixth
# read an EMPTY file, because another property had just truncated it.
#
# The break: the corpora are shared again. The test that says two properties
# do not share a corpus is the gate.
swap crates/engine/tests/crash_properties.rs \
    '        .join(property);' \
    '        .join("shared");'
expect_catch "property: two properties share one corpus" \
    "cargo test -p craftkit-engine --profile ci two_properties_do_not_share_a_corpus"

# ------------------------------------------------- the script's own rules ----
# Section 187. Two of the anchors gate's rules are about the script's own
# text: that a file an anchor names is on the list of files a mutation may
# write to, and that the script checks its anchors before it starts.
#
# Neither could be given a mutation, because the list and the call were
# inside the script -- and bash reads a script as it goes, so a case cannot
# edit the script it is running. The list is a file the script reads, and the
# gate has a --script flag, so both rules can be broken.
#
# The first break: the list loses a file an anchor names. A mutation to that
# file could not be put back, and the gate must say so.
#
# The path becomes a comment rather than disappearing, because a swap that
# removed the text would take the anchor with it and the gate would report
# the missing anchor instead: a case caught for the wrong reason tests
# nothing.
swap scripts/mutation-files.txt \
    'crates/engine/src/crash.rs' \
    '# crates/engine/src/crash.rs'
expect_catch "the file list lost a file an anchor names" \
    "cargo xtask anchors"

# The second break: the list names a file that is not in the tree. The script
# would snapshot nothing for it and the restore would put nothing back.
swap scripts/mutation-files.txt \
    'crates/ui/src/lib.rs' \
    'crates/ui/src/lib.rs
crates/ui/src/not-a-file.rs'
expect_catch "the file list names a file that is not in the tree" \
    "cargo xtask anchors"

# The third break: the list has a blank line. A blank line is not a path,
# and the script refuses it before it does anything else.
swap scripts/mutation-files.txt \
    'crates/core/src/value.rs' \
    'crates/core/src/value.rs
'
expect_catch "the script refuses a list with a blank line" \
    "scripts/mutation-test-gates.sh --check"

# The fourth break: the list names a file that is not in the tree, and the
# script refuses that too -- the gate's rule and the script's rule are the
# same rule, and each door has to notice it.
swap scripts/mutation-files.txt \
    'crates/core/src/value.rs' \
    'crates/core/src/value.rs
crates/core/src/not-a-file.rs'
expect_catch "the script refuses a list with a file that is not in the tree" \
    "scripts/mutation-test-gates.sh --check"

# The fifth break: the list names the same file twice. The same file twice
# is not two files.
swap scripts/mutation-files.txt \
    'crates/core/src/value.rs' \
    'crates/core/src/value.rs
Cargo.lock'
expect_catch "the file list lists the same file twice" \
    "cargo xtask anchors"

# The sixth break: the script does not check its anchors before it starts.
# The copy is what the gate reads, because the script cannot be changed while
# it runs.
broken_copy "$SNAPSHOT/no-preflight.sh" \
    '    check_anchors' \
    '    # check_anchors'
expect_catch "the script does not check its anchors before it starts" \
    "cargo xtask anchors --script $SNAPSHOT/no-preflight.sh"

# The seventh break: the script calls its check, and the check runs nothing.
broken_copy "$SNAPSHOT/no-gate.sh" \
    'out="$(cargo xtask anchors 2>&1)"; rc=$?' \
    'out="$(cargo xtask nothing 2>&1)"; rc=$?'
expect_catch "the script's check does not run the anchors gate" \
    "cargo xtask anchors --script $SNAPSHOT/no-gate.sh"

# ---------------------------------------------------- the time's range ----
# Section 188. A time has three spellings -- seconds, a frame and a tick --
# and the three had three different ranges: the seconds spelling refused an
# instant past one bound, and the other two took the nearest representable
# instant and said nothing. So the same hostile value got three different
# answers depending on how it was written, and a fraction of a frame was
# silently truncated to a different instant than the one that was asked for.
#
# Nothing noticed, because the generator never asked: the trap that makes a
# range's two ends equal replaced the value under test, so the hostile value
# never reached the reader. The sweep now fires every value at every
# parameter, and the values come from the tree's own limits.
#
# The gates named here are the **properties**, one by one, because that is the
# point: a case that ran the whole crate would not say which property caught
# it.

# The first brings the old frame spelling back: the count is taken and the
# nearest representable instant is used instead.
swap crates/core/src/tick.rs \
    '        let ticks = (frames as i128).saturating_mul(per_frame);' \
    '        let ticks = (frames as i128).saturating_mul(per_frame).clamp(MIN_INSTANT_TICKS, MAX_INSTANT_TICKS);'
expect_catch "engine: a frame past the tree's range is taken silently" \
    "cargo test -p craftkit-engine --profile ci an_instant_past_the_trees_range_is_refused_in_every_spelling"

# The second does the same for the tick spelling.
swap crates/core/src/tick.rs \
    '        if !(MIN_INSTANT_TICKS..=MAX_INSTANT_TICKS).contains(&raw) {' \
    '        if !(i128::MIN..=i128::MAX).contains(&raw) {'
expect_catch "engine: a tick past the tree's range is taken silently" \
    "cargo test -p craftkit-engine --profile ci an_instant_past_the_trees_range_is_refused_in_every_spelling"

# The third does the same for the seconds spelling, which is the one that
# used to refuse a range of its own.
swap crates/core/src/tick.rs \
    '        let ticks = Self::ticks_of_seconds(s);' \
    '        let ticks = Self::ticks_of_seconds(s).clamp(MIN_INSTANT_TICKS, MAX_INSTANT_TICKS);'
expect_catch "engine: a second past the tree's range is taken silently" \
    "cargo test -p craftkit-engine --profile ci an_instant_past_the_trees_range_is_refused_in_every_spelling"

# The fourth brings the truncation back: a frame is a count, and a count has
# to be exact.
swap crates/engine/src/params.rs \
    '    if !n.is_finite() || n.fract() != 0.0 {' \
    '    if !n.is_finite() {'
expect_catch "engine: a fraction of a count is truncated again" \
    "cargo test -p craftkit-engine --profile ci a_fraction_of_a_count_is_refused"

# The fifth answers a value past the tree's range in the wrong words, so a
# caller cannot tell a range that is too big from a spelling that is wrong.
swap crates/engine/src/params.rs \
    '            Err(TimeError::Range) => Err(bad(past_the_range(&format!("`{s}`")))),' \
    '            Err(TimeError::Range) => Err(bad(format!("`{s}` is not a time"))),'
expect_catch "engine: the refusal of an instant past the range says something else" \
    "cargo test -p craftkit-engine --profile ci an_instant_past_the_trees_range_is_refused_in_every_spelling"

# The sixth stops the generator from holding the value past the tree's range,
# so the sweep walks a range that is not the tree's.
swap crates/engine/tests/properties.rs \
    '            json!(MAX_MAGNITUDE),
            json!(MAX_MAGNITUDE + 1.0),' \
    '            json!(MAX_MAGNITUDE),'
expect_catch "engine: the generator stops holding the value past the tree's range" \
    "cargo test -p craftkit-engine --profile ci every_command_at_every_parameter_with_every_hostile_value"

# The seventh stops the sweep firing every value, which is the same defect in
# the loop rather than in the list.
swap crates/engine/tests/properties.rs \
    '            for value in values {' \
    '            for value in values.iter().take(1) {'
expect_catch "engine: the sweep stops firing every value" \
    "cargo test -p craftkit-engine --profile ci every_command_at_every_parameter_with_every_hostile_value"

# And the eighth leaves the alternative of the parameter under test in the
# call, so the reader takes the other spelling of the pair and never asks
# about the parameter the property is about.
swap crates/engine/tests/properties.rs \
    '        .or_else(|| spec.params.iter().find(|p| p.alternative == Some(under)).map(|p| p.name));' \
    '        ;'
expect_catch "engine: the reader is never asked about the parameter" \
    "cargo test -p craftkit-engine --profile ci a_value_past_the_trees_range_is_refused_at_every_parameter"

# ------------------------------------------------- the lists of the gates ----
# Section 189. The tree prints what `ci` runs in several places, because the
# reader of each place wants it there. Four of the copies were read by a gate
# and four were not -- and three of those four had drifted: each was missing
# the two gates that were added last, so AGENTS.md, the architecture
# document and the editing skill all told a reader that `ci` runs eight
# gates -- or six -- when it runs eleven.
#
# The gate now reads every copy, and the copies that are not the whole list
# are reported. It also checks the order, which was never checked: a table of
# gates says what each gate protects, and its rows are in the order the gates
# run.

# The first break is the drift itself: a document that names every gate but
# one.
swap AGENTS.md \
    'cargo xtask ci          # fmt, anchors, assets, cleanroom, lists, layers, web, clippy, tests, fuzz, wasm' \
    'cargo xtask ci          # fmt, assets, cleanroom, lists, layers, web, clippy, tests, fuzz, wasm'
expect_catch 'a document says `ci` runs fewer gates than it does' \
    "cargo xtask lists"

# The second break is the order: the table's rows are in the order the gates
# run, and nothing said so.
swap README.md \
    '| `lists` | one deny attribute everywhere; the layer table matches the code *and* `cargo metadata`; sorted lists |
| `layers` | strictly-lower dependencies; no UI toolkit below L6 |' \
    '| `layers` | strictly-lower dependencies; no UI toolkit below L6 |
| `lists` | one deny attribute everywhere; the layer table matches the code *and* `cargo metadata`; sorted lists |'
expect_catch "a document's table of gates lists the gates in the wrong order" \
    "cargo xtask lists"

# ------------------------------------------------------ the script's labels ----
# Section 190. The label is the sentence that says what a case is, and it is
# the only place the log says what was tested. The shell sits between the
# author and the reader, and inside double quotes it expands a dollar and
# runs a backtick -- so the label a reader saw was not always the label the
# author wrote.
#
# Measured: the label of the section 173 case was rewritten in every run
# since section 173, and nothing noticed, because the suite's verdict is a
# number and the label is not part of it.
#
# The first break is a copy of the script with a label the shell would
# rewrite. The copy is what the gate reads, because the script cannot be
# changed while it runs.
broken_copy "$SNAPSHOT/bad-label.sh" \
    "expect_catch 'a document says \`ci\` runs fewer gates than it does' \\" \
    "expect_catch \"a document says \`ci\` runs fewer gates than it does\" \\"
expect_catch 'the gate ignores a label the shell would rewrite' \
    "cargo xtask anchors --script $SNAPSHOT/bad-label.sh"

# The second break is the rule itself: the reader stops saying that the
# shell would change a label. The test that covers the rule must fail.
swap xtask/src/anchors.rs \
    "            '\$' | '\`' => return true," \
    "            '\$' | '\`' => return false,"
expect_catch 'the reader stops saying that the shell would rewrite a label' \
    "cargo test -p xtask --profile ci a_label_in_double_quotes_with_a_backtick_is_reported"

# ------------------------------------------- the script's own interface ----
# Section 191. The script's header is where a reader learns how to run it, and
# it is the script's own copy of its interface. It named two of the three
# ways to run the script, and the one it missed -- `--check` -- is the one
# that answers a question a contributor asks.
#
# And a document in the tree still told a contributor to add their file to
# the script's `FILES` array. The array is rebuilt from
# `scripts/mutation-files.txt` on every run, so a path added to the array is
# lost before the first case runs.
#
# The first break is a copy of the script with a header that misses a way to
# run it. The copy is what the gate reads, because the script cannot be
# changed while it runs.
broken_copy "$SNAPSHOT/no-check.sh" \
    '#   scripts/mutation-test-gates.sh --check       # is the file list usable?
#   scripts/mutation-test-gates.sh --self-test   # exits 1; exits 2 if the self-test is broken' \
    '# The file list is checked with --check.
#   scripts/mutation-test-gates.sh --self-test   # exits 1; exits 2 if the self-test is broken'
expect_catch 'the gate ignores a header that misses a way to run the script' \
    "cargo xtask anchors --script $SNAPSHOT/no-check.sh"

# The second break is the document: the tree names the script's array again.
swap CONTRIBUTING.md \
    "**A mutation's target file must also be in \`scripts/mutation-files.txt\`.**" \
    "**A mutation's target file must also be in \`scripts/mutation-files.txt\`.** The script's \`FILES\` array is the same list, so either is fine."
expect_catch 'the gate ignores a document that names the scripts array' \
    "cargo xtask anchors"

# ---------------------------------------------- the texts the shell writes ----
# Section 192. A text a mutation call writes is text the shell can change: a
# backtick inside double quotes is a command substitution, and a dollar is a
# variable. The gate's reader resolves the four escapes a double quote has
# and stops there, so an anchor the shell would rewrite reads as the text the
# author wrote -- and the run stops twenty minutes in with "MUTATION DID NOT
# APPLY".
#
# The break is a copy of the script with one anchor unescaped. The copy is
# what the gate reads, because the script cannot be changed while it runs.
broken_copy "$SNAPSHOT/unescaped.sh" \
    'in \`scripts/mutation-files.txt\`.**" \' \
    'in `scripts/mutation-files.txt`.**" \'
expect_catch 'the gate ignores a text the shell would rewrite' \
    "cargo xtask anchors --script $SNAPSHOT/unescaped.sh"

# ------------------------------------------------- the copies' own anchors ----
# Section 193. A `broken_copy` names an anchor the way a `swap` does: a text
# it looks for in a file. Section 184 checks every `swap` anchor before the
# run starts, because an anchor that moved is found when the case that uses
# it is reached -- which for these is at the very end of the run, and the
# run then stops with "THE BROKEN COPY DID NOT APPLY" and every result below
# it is about nothing.
#
# The copy replaces the FIRST place its text is in, and the call's own
# argument is a place its text is in. So the rule for a `broken_copy` is
# what it is for a `swap`: ONE place, apart from the call's own.
#
# The break is a copy of the script with one `broken_copy` call written
# where a path in the ephemeral list is. The copy is what the gate reads,
# because the script cannot be changed while it runs. The text the copy
# looks for is written twice -- once in the call and once in the copy's own
# call -- so the copy's outer call stays clean and only the copy's own call
# is broken.
broken_copy "$SNAPSHOT/copy-changes-nothing.sh" \
    '    crates/unlisted' \
    'broken_copy never.sh \
    "abc""def" \
    "    crates/unlisted"'
expect_catch 'the gate ignores a copy whose broken_copy looks for text the script does not hold' \
    "cargo xtask anchors --script $SNAPSHOT/copy-changes-nothing.sh"

# The second break: the call comes before the text it names, so the copy
# replaces the call's own argument and breaks the call and not the script.
broken_copy "$SNAPSHOT/copy-breaks-its-own-call.sh" \
    '    crates/ui/build-probe.py' \
    'broken_copy never.sh \
    "    crates/ui/build-probe.py" \
    "x"'
expect_catch 'the gate ignores a copy whose broken_copy breaks its own call' \
    "cargo xtask anchors --script $SNAPSHOT/copy-breaks-its-own-call.sh"

# The third break: the text is in two places, so the copy breaks whichever
# comes first, and that may not be the place the case meant.
broken_copy "$SNAPSHOT/copy-breaks-the-first-of-two.sh" \
    '    crates/io/src/format/pending/probe.rs' \
    'broken_copy never.sh \
    "    exit 1" \
    "    crates/io/src/format/pending/probe.rs"'
expect_catch 'the gate ignores a copy whose broken_copy breaks the first of two places' \
    "cargo xtask anchors --script $SNAPSHOT/copy-breaks-the-first-of-two.sh"

# ------------------------------------------------ the change hook's order ----
# Section 194. The hook fires before the reply is sent, and that order is the
# whole point of it: a host that draws the session asks for a repaint, and a
# client that read the reply before the window had redrawn would be looking at
# the frame from before the command.
#
# The API's own doc comment said the hook was called after the reply was sent,
# and nothing held the order. Moving the hook left every test in the crate
# green -- the two tests that exercised the hook said how many times it fired
# and not when.
#
# The break swaps the two. The test that holds the order must notice.
swap crates/control/src/server.rs \
    '        if matches!(method, Method::Execute | Method::Ui)
            && let Some(handler) = after
        {
            handler();
        }

        if send(&mut writer, &response).is_err() {
            break;
        }' \
    '        if send(&mut writer, &response).is_err() {
            break;
        }

        if matches!(method, Method::Execute | Method::Ui)
            && let Some(handler) = after
        {
            handler();
        }'
expect_catch 'the change hook fires after the reply is sent' \
    "cargo test -p craftkit-control --profile ci the_change_hook_fires_before_the_reply_is_sent"

# ------------------------------------------------- the report that arrives ----
# Section 195. A report is **input**: the recipe asks for `report.json` from
# whoever had the crash, so the file arrives from another machine -- with no
# crash folder around it and no id to look it up by. Every door in the tree
# read a report by id, out of a folder, so an arriving report had no door at
# all: `run`, `eval` and `snapshot` refused it, and the only way to read one
# was to copy it into the crash folder under its own id, by hand.
#
# The break points the file door at some other file. The test that replays an
# arriving report must notice: it gives the CLI the file and an EMPTY crash
# folder, so nothing but the file can answer.
swap apps/craftkit-cli/src/cli.rs \
    '            let outcome = craftkit_engine::crash::reproduce_file(path)?;' \
    '            let outcome = craftkit_engine::crash::reproduce_file(Path::new("no-such-report.json"))?;'
expect_catch 'the door for a report that arrives in a file reads some other file' \
    "cargo test -p craftkit-cli --profile ci --test crash_pipeline"

# ------------------------------------------------------ the log of a typo ----
# Section 196. The log is the record of what an agent asked for, and a
# mistyped parameter name is the most common thing an agent gets wrong. Every
# other ending of a command was in the log -- the unknown id, the disabled
# command, the command's own refusal, the panic, and the parameters too big
# to trace -- and this one was not: the refusal went back to the caller and
# the log stayed empty.
#
# The break takes the log entry out. The test that holds the entry must
# notice.
swap crates/engine/src/session.rs \
    '        if let Err(source) = p.deny_unknown(&spec.param_names()) {
            let err = EngineError::Param { id: id.to_string(), source };
            self.log.record(id, false, &err.to_string());
            return Err(err);
        }' \
    '        if let Err(source) = p.deny_unknown(&spec.param_names()) {
            let err = EngineError::Param { id: id.to_string(), source };
            return Err(err);
        }'
expect_catch 'engine: a command with a parameter it does not have is not logged' \
    "cargo test -p craftkit-engine --profile ci a_command_with_a_parameter_it_does_not_have"

# ---------------------------------------------------- the kind in the CLI ----
# Section 197. The tree splits its failures in two -- a refusal is a message
# someone can act on, and an internal error is worth a crash report and a
# bug -- and an agent branches on the kind. The control channel's reply has a
# kind field and the MCP's tool error has one; the CLI is the program an
# agent drives, and it printed the message alone.
#
# The break takes the kind out of the CLI's failure line. The test that
# holds it must notice.
swap apps/craftkit-cli/src/cli.rs \
    '    format!("{}: {e}", e.kind())' \
    '    format!("{e}")'
expect_catch 'cli: a failure the CLI prints does not carry the kind' \
    "cargo test -p craftkit-cli --profile ci --test script a_failure_the_cli_prints_carries_the_kind"

# ------------------------------------------- the tool names in a description ----
# Section 198. The `craftkit_commands` tool's description said "for when
# `craftkit_execute` refuses" -- and the server advertises no such tool. The
# commands are advertised one by one -- `craftkit_node_add`,
# `craftkit_prop_set` -- and the registry has no `execute` command, so an
# agent that read the description, looked for the tool and did not find it,
# could conclude the server cannot run commands at all.
#
# The break puts the name back. The test that holds the descriptions must
# notice.
swap crates/mcp/src/tools.rs \
    '                      a command refuses and you want to see what the command actually takes.' \
    '                      `craftkit_execute` refuses and you want to see what the command actually takes.'
expect_catch 'mcp: a description names a tool the server does not advertise' \
    "cargo test -p craftkit-mcp --profile ci every_tool_name_in_a_description_is_a_tool_the_server_advertises"

# --------------------------------------------- the problems panel's scroll ----
# Section 199. The properties panel's scroll area was told not to shrink
# across in section 126, and the node list's in section 128 -- and the
# problems panel was left shrinking to its content, so the wheel did nothing
# over most of the panel: the panel is 884 points wide and the content 244 of
# them.
#
# The break puts the shrink back. The test that holds the wheel must notice.
swap crates/ui/src/shell.rs \
    '            ScrollArea::vertical().max_height(140.0).auto_shrink([false, true]).show(ui, |ui| {' \
    '            ScrollArea::vertical().max_height(140.0).show(ui, |ui| {'
expect_catch 'ui: the problems panel shrinks to its content and the wheel dies over the margin' \
    "cargo test -p craftkit-ui --profile ci the_wheel_scrolls_the_problems_panel_from_across_the_panel"

# ------------------------------------------------ the write side of a report ----
# Section 200. The tree could read a crash report and could not write one
# without a window: the only producer anywhere in the tree was the desktop
# binary's panic hook, so a headless program produced no report at all.
#
# The first break writes a report with no replay in it. The sample session
# wipes its trace -- a session that opens with three entries in its log is a
# lie about what the user did -- so a report built from it would replay
# nothing and "reproduce" itself, and the test that holds the report must
# notice.
swap apps/craftkit-cli/src/cli.rs \
    '    let session = craftkit_engine::Session::sample_recorded();' \
    '    let session = craftkit_engine::Session::sample();'
expect_catch 'cli: the report the CLI writes carries no replay' \
    "cargo test -p craftkit-cli --profile ci --test crash_pipeline the_cli_writes_a_report_the_tree_can_read_and_replay"

# The second break writes a file that is not the report the folder holds. One
# serializer and one atomic writer serve both doors; take the serializer away
# and the two hold different bytes, and the test that holds them together
# must notice.
swap crates/engine/src/crash.rs \
    '    write_atomic(path, &report_json)' \
    '    write_atomic(path, "")'
expect_catch 'crash: the file door and the folder door write different bytes' \
    "cargo test -p craftkit-engine --profile ci --lib the_file_door_and_the_folder_door_write_the_same_bytes"

# The third break takes the door out of the parser. The tree's write side is
# a door an agent drives, and a door the parser does not accept does not
# exist.
swap apps/craftkit-cli/src/args.rs \
    '                Some("write") => {' \
    '                Some("write-disabled") => {'
expect_catch 'cli: the parser stops accepting the door that writes a report' \
    "cargo test -p craftkit-cli --profile ci crashes_write_needs_an_output_and_takes_nothing_else"

# ---------------------------------------------- the fuzzer's third target ----
# Section 201. The tree reads three files and the fuzzer watched two of them:
# the crash report -- the format that arrives from another machine -- had no
# target, because it had no writer, and the writer is what section 200 gave
# it.
#
# The first break takes the report out of the fuzzer's list, so the fuzzer
# watches two doors again. The document's table is checked against the
# fuzzer's own list, and the test that holds the two together must notice.
swap xtask/src/fuzz.rs \
    '    pub const ALL: [Target; 3] = [Target::Design, Target::Script, Target::Report];' \
    '    pub const ALL: [Target; 3] = [Target::Report, Target::Script, Target::Design];'
expect_catch 'fuzz: the fuzzer watches the targets in a different order from the table' \
    "cargo xtask lists"

# The second break gives the report the wrong door -- `show` instead of
# `replay`. The fuzzer would still run, and the seed would still be accepted,
# because `show` reads the report too: the child was wrong and nothing said
# so. The table names the child, and the table is checked against the code.
swap xtask/src/fuzz.rs \
    '                vec!["crashes".to_string(), "replay".to_string(), "--file".to_string()]' \
    '                vec!["crashes".to_string(), "show".to_string(), "--file".to_string()]'
expect_catch 'fuzz: the report target calls a door the table does not name' \
    "cargo xtask lists"

# The third break stops breaking the report's own shape, so the fuzzer's
# structural mutation leaves the file alone and the reader is never pushed
# past its own rules.
swap xtask/src/fuzz.rs \
    '        Target::Report => break_the_report(rng, bytes),' \
    '        Target::Report => bytes.to_vec(),'
expect_catch 'fuzz: the report target leaves the report alone instead of breaking it' \
    "cargo test -p xtask --profile ci the_report_s_shape_is_broken_with_the_file_still_valid_json"

# ------------------------------------------------- a command's parameters ----
# Section 202. Four doors run a command. Two of them -- the control channel
# and the MCP server -- refused a `params` that is not an object, and the
# other two -- the script door and the report door -- read it, and the
# command ran with no parameters at all. The engine's reader treats a
# non-object as an empty view, which is right for a call and wrong for a
# file.
#
# The break takes each door's check away, one at a time. The test that holds
# that door must notice.
swap apps/craftkit-cli/src/cli.rs \
    '        if !step.params.is_object() {' \
    '        if false {'
expect_catch 'cli: the script door runs a step whose parameters are not an object' \
    "cargo test -p craftkit-cli --profile ci a_step_whose_parameters_are_not_an_object_is_refused"

swap crates/engine/src/crash.rs \
    '        if !step.params.is_object() {' \
    '        if false {'
expect_catch 'crash: the report door runs a step whose parameters are not an object' \
    "cargo test -p craftkit-engine --profile ci --lib a_report_whose_step_has_broken_parameters_is_refused"

swap crates/control/src/server.rs \
    '    if !params.is_object() {' \
    '    if false {'
expect_catch 'control: the channel runs a command whose parameters are not an object' \
    "cargo test -p craftkit-control --profile ci an_execute_with_params_that_are_not_an_object_is_refused"

swap crates/mcp/src/server.rs \
    '    if !arguments.is_object() {' \
    '    if false {'
expect_catch 'mcp: the server runs a command whose arguments are not an object' \
    "cargo test -p craftkit-mcp --profile ci a_tool_call_with_arguments_that_are_not_an_object_is_refused"

# ------------------------------------- the view a gate reads the tree in ----
# Section 204. The gates that read the tree's Rust source had no way to say
# whether they were asking about the code the tree ships or about the file as
# it is. The tree now has one way to open a Rust file -- `read_rust(path,
# subject)` -- and a list of the gates that do, with the subject each one
# reads for. The list is a promise about the code, and a gate holds it up.
#
# The first break changes a gate's mind: it reads the file as it is now, and
# the list still says it reads the code the tree ships.
swap xtask/src/lists.rs \
    '        let Ok(text) = read_rust(&path, Subject::Production) else { continue };
        for name in the_tree_s_bounds(&code_lines_kept(&text)) {' \
    '        let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
        for name in the_tree_s_bounds(&code_lines_kept(&text)) {'
expect_catch 'lists: a gate reads the tree'"'"'s Rust source for a view the list does not name' \
    "cargo xtask lists"

# The second break takes a gate out of the list -- or puts a name in it that
# opens no Rust file at all. Either way the list and the code have parted
# company, and the gate that holds the list up must notice.
swap xtask/src/lists.rs \
    '    ("ui_structs", Subject::Production),' \
    '    ("ui_structs_renamed", Subject::Production),'
expect_catch 'lists: the list of the gates that read Rust does not match the code' \
    "cargo xtask lists"

# The third break gives the tree a third way to read a file. The tree has
# two -- `read_text` and `read_rust` -- and a Rust file is read with a
# subject, so a gate cannot read one by accident.
swap xtask/src/lists.rs \
    '/// The name of the top-level function a line opens, if it opens one.
fn top_level_fn_name(line: &str) -> Option<&str> {' \
    '/// A third way to read a file.
fn a_third_reader(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// The name of the top-level function a line opens, if it opens one.
fn top_level_fn_name(line: &str) -> Option<&str> {'
expect_catch 'lists: the tree has a third way to read a file' \
    "cargo xtask lists"

# The fourth break puts the rows of a table back together. The columns of a
# table are different things, so a claim in one row and a file in the next
# are not about each other -- and the tree'"'"'s own playbook says
# "`Session::execute` does the rest" in the third column of one row while the
# next row names a file. The claim was false, and it was true only because
# the file'"'"'s test module said `execute`.
swap xtask/src/lists.rs \
    '                if let (Some(a), Some(b)) = (rows.row_of(left.0), rows.row_of(right.0))
                    && a != b
                {
                    continue;
                }' \
    '                if false {
                    continue;
                }'
expect_catch 'lists: a claim in one row of a table is paired with a file in the next' \
    "cargo xtask lists"

restore
finish
