#!/bin/bash
echo "Universal Search Service Status"
echo "==============================="

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SERVICE_NAME=universal-search

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    systemctl --user status "$SERVICE_NAME" 2>/dev/null | head -5
elif [[ "$OSTYPE" == "darwin"* ]]; then
    launchctl list | grep universalsearch || echo "Service not registered"
fi

echo ""
echo "Health check:"
if command -v curl &>/dev/null; then
    curl -sf http://localhost:3005/health && echo "Port 3005: OPEN" || echo "Port 3005: CLOSED"
else
    timeout 3 bash -c "cat < /dev/null > /dev/tcp/localhost/3005" 2>/dev/null && echo "Port 3005: OPEN" || echo "Port 3005: CLOSED"
fi
