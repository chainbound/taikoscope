#!/bin/sh

# Exit on any error
set -e

echo "🔗 Starting Tailscale daemon..."

TS_STATE="/var/lib/tailscale/tailscaled.state"
TS_SOCKET="/var/run/tailscale/tailscaled.sock"
TS_OPTS=""

if [ ! -c /dev/net/tun ]; then
  echo "❗ /dev/net/tun not found, using userspace networking"
  TS_OPTS="--tun=userspace-networking"
fi

/app/tailscaled --state=${TS_STATE} --socket=${TS_SOCKET} ${TS_OPTS} &

# Wait a moment for tailscaled to start
sleep 2

echo "🌐 Connecting to Tailscale network..."
if [ -n "${TAILSCALE_AUTHKEY}" ]; then
  /app/tailscale up --auth-key=${TAILSCALE_AUTHKEY} --hostname=taikoscope-hekla --accept-routes
else
  echo "⚠️  TAILSCALE_AUTHKEY not set. Skipping tailscale up."
fi

echo "✅ Tailscale connected. Starting Taikoscope..."
exec /app/taikoscope
