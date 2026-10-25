#!/bin/sh
# Drives a test copy of the bundled app in target/try-home, next to any real one.
#   try.sh start <file.md>   launch it on that rundown, strip at frame $FRAME
#   try.sh send <action>...  run actions as the menus do: Next, 'EditCard "Delete"'
#   try.sh shot <out.png>    capture the strip
#   try.sh stop
set -eu
cd "$(dirname "$0")/.."
home=$PWD/target/try-home
frame=${FRAME:-0 400 1200 150}
app=target/release/KeyPrompter.app
alive() { [ -e "$home/pid" ] && kill -0 "$(cat "$home/pid")" 2>/dev/null; }
case $1 in
start)
  "$0" stop
  rm -rf "$home" && mkdir -p "$home" && mkfifo "$home/drive"
  printf 'frame %s\ncapture shown\n' "$frame" > "$home/state.txt"
  open -n --env KEYPROMPTER_HOME="$home" "$app" --args "$(realpath "$2")"
  for _ in 1 2 3 4 5 6 7 8 9 10; do alive && break; sleep 0.5; done
  alive || { echo "test app did not start" >&2; exit 1; }
  sleep 2
  ;;
send)
  shift
  alive || { echo "test app is not running" >&2; exit 1; }
  printf '%s\n' "$@" > "$home/drive"
  sleep 1
  ;;
shot)
  alive || { echo "test app is not running" >&2; exit 1; }
  id=$(osascript -l JavaScript scripts/windows.js KeyPrompter "$(cat "$home/pid")")
  screencapture -x -o -l"$id" "$2"
  ;;
stop)
  if alive; then kill "$(cat "$home/pid")"; fi
  rm -f "$home/pid"
  ;;
esac
