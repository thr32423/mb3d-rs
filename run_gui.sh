#!/bin/sh
# builds mb3d if needed, starts the browser editor and opens it in the default browser.
# extra arguments go to `mb3d gui`, e.g. ./run_gui.sh --formulas DIR --maps DIR file.m3p
# the port can be changed with PORT=8081 ./run_gui.sh
cd "$(dirname "$0")" || exit 1
PORT=${PORT:-8080}
cargo build --release || exit 1
./target/release/mb3d gui --port "$PORT" "$@" &
pid=$!
trap 'kill $pid 2>/dev/null' INT TERM
# wait until the server answers (at most 10 s), then open the page
i=0
while ! curl -s -o /dev/null "http://127.0.0.1:$PORT/"; do
    kill -0 $pid 2>/dev/null || exit 1
    i=$((i + 1)); [ $i -gt 100 ] && break
    sleep 0.1
done
xdg-open "http://127.0.0.1:$PORT/" >/dev/null 2>&1 &
wait $pid
