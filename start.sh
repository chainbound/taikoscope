#!/bin/sh

# Exit on any error
set -e

echo "🔗 Starting Tailscale daemon..."
/app/tailscaled --state=/var/lib/tailscale/tailscaled.state --socket=/var/run/tailscale/tailscaled.sock &

# Wait a moment for tailscaled to start
sleep 2

echo "🌐 Connecting to Tailscale network..."
/app/tailscale up --auth-key=${TAILSCALE_AUTHKEY} --hostname=taikoscope-hekla --accept-routes

echo "✅ Tailscale connected. Starting Taikoscope..."
exec /app/taikoscope