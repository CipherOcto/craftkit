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
#    wrong. For the same reason, do not edit a file in `FILES` while this
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
#   scripts/mutation-test-gates.sh --self-test   # expected to exit non-zero

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Every file a mutation may write to.
readonly FILES=(
    Cargo.toml
    ATTRIBUTION.md
    docs/architecture.md
    crates/core/Cargo.toml
    crates/core/src/lib.rs
    crates/core/src/readers.rs
    crates/core/src/value.rs
    crates/core/src/tick.rs
    crates/model/Cargo.toml
    crates/model/src/lib.rs
    crates/model/src/property.rs
    crates/io/src/lib.rs
    crates/io/src/format.rs
    crates/io/src/store.rs
    crates/io/src/format/pending.rs
    crates/kernel/src/lib.rs
    crates/ui/Cargo.toml
    crates/ui/src/lib.rs
    crates/ui/src/automation.rs
    crates/ui/src/shell.rs
    crates/ui/src/raster.rs
    crates/mcp/src/server.rs
    crates/mcp/src/rpc.rs
    crates/mcp/src/tools.rs
    crates/engine/src/crash.rs
    crates/engine/src/command.rs
    crates/engine/src/commands.rs
    crates/engine/src/history.rs
    crates/engine/src/params.rs
    crates/engine/src/lib.rs
    crates/engine/src/log.rs
    crates/engine/src/session.rs
    crates/control/src/server.rs
    crates/control/src/protocol.rs
    crates/control/src/token.rs
    crates/model/src/keyframe.rs
    crates/model/src/document.rs
    crates/model/src/fingerprint.rs
    crates/model/src/path.rs
    crates/eval/src/evaluate.rs
    apps/craftkit/src/main.rs
    apps/craftkit-cli/src/main.rs
    apps/craftkit-cli/src/args.rs
    apps/craftkit-cli/src/cli.rs
    apps/craftkit-web/src/main.rs
    xtask/src/ci.rs
    xtask/src/doctor.rs
    xtask/src/lists.rs
    docs/recipes/add-an-icon.md
    docs/recipes/change-the-file-format.md
    docs/recipes/fix-a-crash.md
    docs/design/file-format.md
    docs/patterns/clone-before-edit.md
    docs/patterns/everything-is-a-command.md
    docs/agent-playbook.md
    AGENTS.md
    README.md
    apps/craftkit-web/index.html
    xtask/src/wasm.rs
    apps/craftkit-web/sw.js
    scripts/live-window-check.sh
)
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

readonly GREEN=$'\033[32m'
readonly RED=$'\033[31m'
readonly YELLOW=$'\033[33m'
readonly RESET=$'\033[0m'

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

# A checksum of the mutable surface, so the script can prove it changed nothing.
fingerprint() {
    for f in "${FILES[@]}"; do
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
        "  It is not in FILES, so snapshot() never copied it, restore() cannot put it" \
        "  back and fingerprint() never looks at it. Add it to FILES first." >&2
    exit 1
}

declare -a CAUGHT=()
declare -a MISSED=()
declare -a BROKE=()
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
expect_catch() {
    local label="$1" gate="$2" compile_is_catch="${3:-no}"
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
    if [[ ${#MISSED[@]} -eq 0 && ${#BROKE[@]} -eq 0 ]]; then
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

    if [[ "$(fingerprint)" != "$BEFORE" ]]; then
        printf '\n%sThe script left the tree changed.%s That is a bug in this script.\n' "$RED" "$RESET"
        status=1
    else
        printf '\nTree unchanged.\n\n'
    fi
    exit $status
}

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
    '    session.load_document(checked_document(report.base)?);' \
    '    let _ = checked_document(report.base)?;'
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
    '            if outcome.reproduced {' \
    '            if true {'
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
    '    let path = dir.join(sanitize(id)).join("report.json");
    let body = read_bounded(&path, MAX_REPORT_BYTES)?;' \
    '    let path = dir.join(sanitize(id)).join("report.json");
    let body = std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;'
expect_catch "crash: a report file of any size is read" \
    "cargo test -p craftkit-engine --profile ci larger_than_the_limit"

# And the same for the steps, which the replay *runs*. The cost is quadratic,
# so the bound has to be checked before the first step, not after.
swap crates/engine/src/crash.rs \
    '    if report.replay.len() > MAX_REPLAY_STEPS {' \
    '    if false {'
expect_catch "crash: a replay runs as many steps as the report holds" \
    "cargo test -p craftkit-engine --profile ci too_many_steps"

# And the base has to be a document the model would accept. The design reader
# refuses a document with two nodes sharing an id; this one did not.
swap crates/engine/src/crash.rs \
    '    doc.validate().map_err(|e| format!("the report'"'"'s base design is not usable: {e}"))?;' \
    ''
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
    '    let body = read_bounded(&path, MAX_REPORT_BYTES)?;
    serde_json::from_str(&body).map_err(|e| format!("{} is not a readable report: {e}", path.display()))' \
    '    let body = read_bounded(&path, MAX_REPORT_BYTES)?;
    let tree: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("{} is not a readable report: {e}", path.display()))?;
    serde_json::from_value(tree).map_err(|e| format!("{} is not a readable report: {e}", path.display()))'
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
    "cargo test -p craftkit-io --profile ci a_document_too_large_to_read_back_is_not_written"

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
        let rate = s.document().meta.frame_rate();' \
    '    run: |s, p| {
        s.doc_mut().meta.name = "written by a read-only command".to_string();
        let rate = s.document().meta.frame_rate();'
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
                        let err = EngineError::Internal {' \
    '                        let err = EngineError::Internal {'
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
    '    restrict(&dir, true);' \
    '    let _ = &dir;'
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
    Phase { name: "wasm", run: wasm::run },
' ''
expect_catch "ci: the tests and wasm phases vanish from the run" \
    "cargo test -p xtask --profile ci the_phases_are_the_documented_ones_and_come_first"

# The closing line itself: naming a phase that did not run is the whole defect,
# so put it back the way it was and check the summary catches it.
swap xtask/src/ci.rs '        crate::Outcome::Ran => ran.push(name),
        crate::Outcome::Skipped => skipped.push(name),' \
    '        crate::Outcome::Ran => ran.push(name),
        crate::Outcome::Skipped => ran.push(name),'
expect_catch "ci: a skipped phase is counted as one that ran" \
    "cargo test -p xtask --profile ci a_phase_that_could_not_run_is_not_counted_as_one_that_ran"

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
    '        if p.raw("start").is_none() && p.raw("end").is_none() && p.raw("fps").is_none() {' \
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
# The anchor is the pair of flags, and it has to be: a longer one would run
# into the apostrophe in "the plan's rate", and an apostrophe closes the
# shell's single-quoted string. The pair is not unique -- four commands have
# it -- and `swap` replaces the first, which is `playback.seek`. That is
# enough: whichever command it lands on, a command that does not mutate and
# claims to be undoable breaks the rule.
swap crates/engine/src/commands.rs \
    '    mutates: false,
    undoable: false,' \
    '    mutates: false,
    undoable: true,'
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
    '            name: "craftkit_crashes".to_string(),' \
    '            name: "craftkit_crashs".to_string(),'
expect_catch "mcp: the crash reports are published under a name nobody calls" \
    "cargo test -p craftkit-mcp --profile ci the_crash_reports_are_a_tool"

# The tool ignored the action it was given, so every request was a list. §113.
swap crates/mcp/src/server.rs \
    '            let action = arguments.get("action").and_then(Json::as_str).unwrap_or("list");' \
    '            let action = "list";'
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
    '        self.clamp_playhead_into_range();
        self.recompute();' \
    '        self.recompute();'
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

restore
finish