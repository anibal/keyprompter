#!/bin/sh
# Drives a test copy of the bundled app in its own profile, next to any real one.
#   try.sh start <file.md>   launch it on that rundown, strip at frame $FRAME
#   try.sh send <Action>...  run actions by name, as in the menus (Next, DuplicateCard)
#   try.sh shot <out.png>    capture the strip
#   try.sh stop
set -eu
cd "$(dirname "$0")/.."
home=${KEYPROMPTER_HOME:-$PWD/target/try-home}
frame=${FRAME:-0 400 1200 150}
app=target/release/KeyPrompter.app
case $1 in
start)
  [ -e "$home/pid" ] && "$0" stop
  rm -rf "$home" && mkdir -p "$home" && mkfifo "$home/drive"
  printf 'frame %s\ncapture shown\n' "$frame" > "$home/state.txt"
  before=$(pgrep -x KeyPrompter || true)
  open -n --env KEYPROMPTER_HOME="$home" "$app" --args "$(realpath "$2")"
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    pid=$(pgrep -x KeyPrompter | grep -vxF "$before" | head -1 || true)
    [ -n "$pid" ] && break
    sleep 0.5
  done
  [ -n "$pid" ] || { echo "test app did not start" >&2; exit 1; }
  echo "$pid" > "$home/pid"
  sleep 2
  ;;
send)
  shift
  printf '%s\n' "$@" > "$home/drive"
  sleep 1
  ;;
shot)
  id=$(osascript -l JavaScript scripts/windows.js KeyPrompter "$(cat "$home/pid")")
  screencapture -x -o -l"$id" "$2"
  ;;
stop)
  kill "$(cat "$home/pid")" 2>/dev/null || true
  rm -f "$home/pid"
  ;;
esac
