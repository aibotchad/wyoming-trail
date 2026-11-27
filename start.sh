#!/usr/bin/env bash
# Build Wyoming Trail and serve it to this computer and the local network.
#
#   ./start.sh            serve on port 8080
#   PORT=9000 ./start.sh  serve on another port
#   ./start.sh --open     also open a browser here
#   RELOAD=1 ./start.sh   developing: reload open pages after each change
#
# By default an open game is never reloaded, so nobody is thrown back to the
# title screen mid-choice when a file changes. Refresh the page to update.
#
# Anyone on the same network can reach the game while this is running.
set -euo pipefail
cd "$(dirname "$0")"

port="${PORT:-8080}"
command -v trunk >/dev/null || PATH="$HOME/.cargo/bin:$PATH"
command -v trunk >/dev/null || {
  echo "Trunk is not installed. Run: cargo install trunk --locked" >&2
  exit 1
}

# The address this machine uses to reach the network (nothing is sent).
lan_ip="$(ip -4 route get 1.1.1.1 2>/dev/null | sed -n 's/.* src \([0-9.]*\).*/\1/p' | head -n 1 || true)"

echo
echo "  On this computer:  http://127.0.0.1:$port/"
if [ -n "$lan_ip" ]; then
  echo "  On your network:   http://$lan_ip:$port/"
else
  echo "  On your network:   no network address found"
fi
echo "  Press Ctrl+C to stop."
echo

reload=(--no-autoreload=true)
if [ "${RELOAD:-0}" = 1 ]; then reload=(); fi

# Trunk rejects the NO_COLOR variable that some terminals set.
# It builds into its own folder, so a plain `trunk serve` running alongside
# can't overwrite what is being served here.
exec env -u NO_COLOR trunk serve --release --dist target/dist-lan \
  --address 0.0.0.0 --port "$port" "${reload[@]}" "$@"
