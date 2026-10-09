#!/bin/sh
# starts the browser editor and opens it in the default browser: in a release
# folder it runs the mb3d next to it, in the source tree it builds mb3d first.
# extra arguments go to `mb3d gui`, e.g. ./run_gui.sh --formulas DIR --maps DIR file.m3p
# the port can be changed with PORT=8081 ./run_gui.sh
cd "$(dirname "$0")" || exit 1
PORT=${PORT:-8080}
if [ -x ./mb3d ]; then
    mb3d=./mb3d
else
    cargo build --release || exit 1
    mb3d=./target/release/mb3d
fi
"$mb3d" gui --port "$PORT" "$@" &
pid=$!
trap 'kill $pid 2>/dev/null' INT TERM
# wait until the server answers (at most 10 s), then open the page
i=0
while ! curl -s -o /dev/null "http://127.0.0.1:$PORT/"; do
    kill -0 $pid 2>/dev/null || exit 1
    i=$((i + 1)); [ $i -gt 100 ] && break
    sleep 0.1
done
url="http://127.0.0.1:$PORT/"
if command -v xdg-open >/dev/null; then xdg-open "$url" >/dev/null 2>&1 &
elif command -v open >/dev/null; then open "$url"
else echo "open $url in a browser"; fi
wait $pid
