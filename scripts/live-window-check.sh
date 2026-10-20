#!/usr/bin/env bash
# Check that the window follows the document.
#
# The defect this exists for: a command that arrived over the control channel
# changed the document on a socket thread, and the window went on showing the
# frame it had painted before. The engine said five nodes; the window said
# three. Nothing errored. An agent that drove the app and then looked at the
# window was looking at a picture of the past.
#
# There is no unit test for this, and there cannot be one. egui offers no
# observable that separates "a repaint was requested" from "a repaint is
# pending" on a context with no window behind it — `has_requested_repaint` is
# already true after the first headless pass, and the wake-up callback only
# fires on a *lowered* delay, which only a real integration loop reaches. Both
# were measured. So this script measures the thing that matters instead: it
# runs the real binary on a real display and compares the window's own pixels
# before and after a command.
#
#   scripts/live-window-check.sh
#
# Exit 0 passed, 1 failed, 77 skipped. 77 is deliberate and is never a pass:
# CI has no display, and a check that quietly reported success there would be
# the same lie as the bug.
#
# `scripts/live-window-check.sh --self-test` exercises the part of this script
# that decides *which* window to measure, without a display, a build or an
# application. §56.

set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

readonly PROFILE=ci
readonly PORT=${CRAFTKIT_LIVE_PORT:-47711}
readonly WORK=$(mktemp -d)
# How many nodes to add. Four, because a two-row difference is small enough
# that noise could be mistaken for it.
readonly ADDED=4
# Differing pixels below this are treated as "the window did not change". A new
# row is a row of text plus a slider: comfortably over 1000, while a repaint
# that changed nothing is exactly 0.
readonly MIN_DIFF_PIXELS=1000

fail() { echo "live-window: FAIL — $*" >&2; exit 1; }
skip() { echo "live-window: SKIP — $*" >&2; exit 77; }
note() { echo "live-window: $*"; }

cleanup() {
    [ -n "${APP_PID:-}" ] && kill "$APP_PID" 2>/dev/null
    rm -rf "$WORK"
}
trap cleanup EXIT

# ------------------------------------------------------- which window, exactly
# The window this check measures has to be *this* app's window.
#
# Two things used to go wrong, and both of them produced a quiet wrong answer:
#
# 1. **The name was not enough.** A second instance of the application has a
#    window with the same name. The check took the first one it found, which
#    is whichever the X server listed first — so with a leftover instance
#    running it measured that instance's window, which never received the
#    commands, and reported "the engine changed and the window did not".
#    That is a false failure with a confident diagnosis, and it is the
#    diagnosis this script exists to make.
#
# 2. **The id was extracted too loosely.** `grep -o '0x[0-9a-f]\+'` matched
#    every hex-looking token on the line, and the geometry on the same line
#    (`1180x720+10+45`) contains `0x720` — a width, not a window.
#
# The pid tells the windows apart: an X window carries `_NET_WM_PID`, the
# process that owns it.

# Every window named "craftkit", one "id pid" pair per line.
#
# The id is the *first field* of the line, not every hex token on it, which is
# what keeps the geometry out of it.
list_windows() {
    local id pid
    while read -r id _; do
        [ -n "$id" ] || continue
        pid=""
        if command -v xprop >/dev/null 2>&1; then
            pid=$(xprop -id "$id" _NET_WM_PID 2>/dev/null | awk '{print $3}')
        fi
        printf '%s %s\n' "$id" "${pid:-}"
    done < <(xwininfo -root -tree 2>/dev/null | awk '$2 == "\"craftkit\":" { print $1 }')
}

# The window whose pid is $1, read from "id pid" lines on stdin.
#
#   0 — the id was written out
#   1 — there are no candidates at all; the caller should wait
#   2 — there are candidates, but none of them is the app's; the caller
#       must not guess, because guessing is the defect
pick_window() {
    local want="$1" id pid
    local -a ids=() pids=()
    while read -r id pid; do
        [ -n "$id" ] || continue
        ids+=("$id")
        pids+=("${pid:-}")
    done
    [ "${#ids[@]}" -gt 0 ] || return 1
    local i
    for i in "${!ids[@]}"; do
        if [ -n "${pids[$i]}" ] && [ "${pids[$i]}" = "$want" ]; then
            printf '%s\n' "${ids[$i]}"
            return 0
        fi
    done
    # One candidate, and no way to tell whose it is: use it. More than one,
    # and none of them is ours: the answer is not knowable, and a check that
    # picks one anyway is a check that measures the wrong thing.
    if [ "${#ids[@]}" -eq 1 ] && [ -z "${pids[0]}" ]; then
        printf '%s\n' "${ids[0]}"
        return 0
    fi
    return 2
}

# Wait for the app's token, and not for the one the last run left behind.
#
# The token file outlives the process that wrote it, so "the file exists" is
# not "the file is this app's". The wait is for the file to hold a token that
# is not the one that was already there.
#
# Prints the token. Status 0 = found, 1 = timed out, 2 = the app is gone.
wait_for_token() { # path old-token pid [tries]
    local path="$1" old="$2" pid="$3" tries="${4:-60}"
    local now
    for _ in $(seq 1 "$tries"); do
        if [ -s "$path" ]; then
            now=$(cat "$path")
            if [ "$now" != "$old" ]; then
                printf '%s' "$now"
                return 0
            fi
        fi
        kill -0 "$pid" 2>/dev/null || return 2
        sleep 0.5
    done
    return 1
}

# ------------------------------------------------------------------- self-test
# The part of this script that can be checked without a display.
#
# A gate that cannot fail is worse than no gate (§1), and the rule this
# script enforces — "the window you measured was this app's window" — is
# exactly the kind of rule that quietly stops holding.
self_test() {
    local fails=0
    expect() { # name expected-status expected-output actual-status actual-output
        if [ "$2" = "$4" ] && [ "$3" = "$5" ]; then
            printf '  ok    %s\n' "$1"
        else
            printf '  FAIL  %s — expected status %s output %s, got status %s output %s\n' \
                "$1" "$2" "$3" "$4" "$5"
            fails=$((fails + 1))
        fi
    }
    run() { # input want -> sets STATUS and OUT
        OUT=$(printf '%s' "$1" | pick_window "$2")
        STATUS=$?
    }

    # The window that belongs to this app, among several that do not.
    run '0xa 111
0xb 222
0xc 333
' 222
    expect "the window whose pid is ours wins" 0 "0xb" "$STATUS" "$OUT"

    # A window with no pid at all, and only one candidate: there is nothing to
    # confuse it with.
    run '0xd
' 222
    expect "a lone window with no pid is used" 0 "0xd" "$STATUS" "$OUT"

    # One candidate, but it belongs to somebody else. Using it would measure
    # the wrong window.
    run '0xe 111
' 222
    expect "a window that belongs to another process is refused" 2 "" "$STATUS" "$OUT"

    # Several candidates, none of them ours.
    run '0xf 111
0x10 333
' 222
    expect "several windows and none of them ours is refused" 2 "" "$STATUS" "$OUT"

    # No window at all: the caller should wait rather than fail.
    run '' 222
    expect "no window at all means wait" 1 "" "$STATUS" "$OUT"

    # The token file outlives the process that wrote it, so the file from the
    # last run is still there when the app starts. Waiting for the file to
    # *exist* would read that one — and the server would refuse it. §86.
    TOKDIR=$(mktemp -d)
    printf 'old-token' > "$TOKDIR/1234.token"
    ( sleep 0.7; printf 'new-token' > "$TOKDIR/1234.token" ) 2>/dev/null &
    NEW=$(wait_for_token "$TOKDIR/1234.token" "old-token" $$ 20)
    expect "a token left over from the last run is not this app's" 0 "new-token" "$?" "$NEW"

    # And a file that never changes is not a token either: the wait gives up.
    STALE=$(wait_for_token "$TOKDIR/1234.token" "new-token" $$ 2)
    expect "a token that never changes is not waited for" 1 "" "$?" "$STALE"
    rm -rf "$TOKDIR"

    if [ "$fails" -gt 0 ]; then
        printf 'live-window: self-test FAILED — %s check(s)\n' "$fails" >&2
        return 1
    fi
    printf 'live-window: self-test ok\n'
    return 0
}

if [ "${1:-}" = "--self-test" ]; then
    self_test
    exit $?
fi

# ---------------------------------------------------------------- preconditions
[ -n "${DISPLAY:-}" ] || skip "no DISPLAY; this check needs a real window"
command -v xwininfo >/dev/null || skip "xwininfo is not installed"
command -v import    >/dev/null || skip "ImageMagick's import is not installed"
command -v compare   >/dev/null || skip "ImageMagick's compare is not installed"
command -v python3   >/dev/null || skip "python3 is not installed"

# --------------------------------------------------------------- build and run
note "building apps/craftkit"
if ! cargo build --quiet -p craftkit --profile "$PROFILE" 2>"$WORK/build.log"; then
    cat "$WORK/build.log" >&2
    fail "the app did not build"
fi

APP="$PWD/target/$PROFILE/craftkit"
[ -x "$APP" ] || fail "no binary at $APP"

note "starting the app on 127.0.0.1:$PORT"
"$APP" --sample --control "$PORT" >"$WORK/app.log" 2>&1 &
APP_PID=$!

TOKEN_FILE="$HOME/.local/share/craftkit/control/$PORT.token"
# The token file **outlives the process that wrote it** — the control server
# replaces it on the next launch and never removes it. So the file from the
# last run is still there when this app starts, and waiting for the file to
# *exist* would read that one, send it, and watch the server refuse it and
# close the connection without a reply. Measured: the first run after a
# previous one failed with "the commands over the control channel did not all
# succeed", and the driver had received nothing at all. §86.
#
# So the wait is for the file to hold something *else*.
OLD_TOKEN=""
[ -s "$TOKEN_FILE" ] && OLD_TOKEN=$(cat "$TOKEN_FILE")
TOKEN=$(wait_for_token "$TOKEN_FILE" "$OLD_TOKEN" "$APP_PID")
TOKEN_STATUS=$?
if [ "$TOKEN_STATUS" -ne 0 ]; then
    kill -0 "$APP_PID" 2>/dev/null || fail "the app exited: $(cat "$WORK/app.log")"
    fail "no new token at $TOKEN_FILE after 30s"
fi

# The window that belongs to *this* app — found by pid, not by name, because a
# second instance has a window with the same name and measuring that one would
# report a confident, wrong diagnosis. See `pick_window` above and §56.
#
# "Not ours" is not "not there": the app's window is mapped a moment after the
# process starts, so an early look sees only the other instance's window. The
# loop waits; only the timeout turns that into a failure.
WID=""
CANDIDATES=""
for _ in $(seq 1 60); do
    CANDIDATES=$(list_windows)
    WID=$(printf '%s\n' "$CANDIDATES" | pick_window "$APP_PID")
    [ $? -eq 0 ] && [ -n "$WID" ] && break
    WID=""
    sleep 0.5
done
[ -n "$WID" ] || fail "no window belonging to this app (pid $APP_PID) appeared; the windows named craftkit are:
$CANDIDATES"
note "window $WID (pid $APP_PID)"

# Sleep until the window has actually been mapped and painted something, so the
# "before" capture is not of a blank frame.
sleep 2

# ------------------------------------------------------- the pixels: before first
# This has to happen BEFORE the driver sends anything. Capturing "before" after
# the commands would compare two post-command frames, report a difference of
# zero, and blame the window for a bug that had already been fixed — which is
# what the first version of this script did.
import -window "$WID" "$WORK/before.png" 2>/dev/null || fail "could not capture the window (before)"
[ -s "$WORK/before.png" ] || fail "the before capture is empty"

# The driver: it asks the engine for its state, adds nodes, asks again.
python3 - "$PORT" "$TOKEN" "$WORK" "$ADDED" <<'PY'
import json, socket, sys
port, token, work, added = int(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4])
s = socket.create_connection(("127.0.0.1", port)); s.settimeout(10)
s.sendall((json.dumps({"id": 1, "token": token, "method": "engine.execute",
                       "params": {"command": "session.stats", "params": {}}}) + "\n").encode())
before = json.loads(s.recv(65536).decode().splitlines()[0])
lines = [json.dumps({"id": 2 + i, "method": "engine.execute",
                     "params": {"command": "node.add",
                                "params": {"name": f"Live {i}", "kind": "filter"}}}) + "\n"
         for i in range(added)]
lines.append(json.dumps({"id": 2 + added, "method": "engine.execute",
                         "params": {"command": "session.stats", "params": {}}}) + "\n")
s.sendall("".join(lines).encode())
buf = b""
while buf.count(b"\n") < added + 1:
    chunk = s.recv(65536)
    if not chunk:
        break
    buf += chunk
s.close()
replies = [json.loads(l) for l in buf.decode().splitlines() if l.strip()]
stats = replies[-1]["data"]
print(f"engine: {before['data']['nodes']} nodes before, {stats['nodes']} after, "
      f"{stats['failures']} failure(s)")
open(f"{work}/engine_after.txt", "w").write(str(stats["nodes"]))
if not all(r["ok"] for r in replies):
    sys.exit("a command was refused: " + json.dumps(replies)[:400])
PY
[ $? -eq 0 ] || fail "the commands over the control channel did not all succeed"

# The engine is the control. The window is the thing that was broken.
AFTER_NODES=$(cat "$WORK/engine_after.txt")
EXPECTED=$((3 + ADDED))
[ "$AFTER_NODES" = "$EXPECTED" ] || fail "engine reports $AFTER_NODES nodes, expected $EXPECTED"

note "the engine says $AFTER_NODES nodes; now looking at the window"

# ------------------------------------------------------------------- the pixels
# The window is given a moment to paint the change it was asked for.
sleep 2
import -window "$WID" "$WORK/after.png"  2>/dev/null || fail "could not capture the window (after)"
[ -s "$WORK/after.png"  ] || fail "the after capture is empty"

# `compare -metric AE` prints the count of differing pixels and exits non-zero
# when they differ, which is not an error here.
DIFF=$(compare -metric AE "$WORK/before.png" "$WORK/after.png" null: 2>&1 || true)
case "$DIFF" in
    ''|*[!0-9]*) fail "could not read the pixel difference (got '$DIFF')" ;;
esac

# Captures outlive the run on purpose: the exit trap deletes the scratch
# directory, and a check that deletes its own evidence is a check nobody can
# look at afterwards. Under target/, which is already ignored.
readonly KEEP="$PWD/target/live-window-check"
mkdir -p "$KEEP"
cp "$WORK/before.png" "$KEEP/before.png"
cp "$WORK/after.png"  "$KEEP/after.png"
note "captures in $KEEP"

if [ "$DIFF" -lt "$MIN_DIFF_PIXELS" ]; then
    note "$DIFF differing pixels (< $MIN_DIFF_PIXELS)"
    note "the engine changed and the window did not — this is the stale-window bug"
    exit 1
fi

note "$DIFF differing pixels — the window followed the document"
exit 0