#!/bin/bash
# Start both bridge and app
set -e
cd "$(dirname "$0")"

echo "Starting bridge..."
(cd bridge && npm run dev) &
BRIDGE_PID=$!

echo "Starting app..."
(cd app && npm run dev) &
APP_PID=$!

echo "Bridge running (PID $BRIDGE_PID) on http://127.0.0.1:4000"
echo "App running (PID $APP_PID) on http://localhost:3000"
echo ""
echo "Press Ctrl+C to stop both servers."

trap "kill $BRIDGE_PID $APP_PID 2>/dev/null" EXIT
wait
