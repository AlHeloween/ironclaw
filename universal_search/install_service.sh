#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SERVICE_BIN="$SCRIPT_DIR/universal-search-service"

echo "============================================"
echo "  Installing Universal Search Service (Linux)"
echo "============================================"
echo

if [ ! -f "$SERVICE_BIN" ]; then
    echo "ERROR: universal-search-service not found at $SERVICE_BIN"
    echo "Please build first: ./build.sh"
    exit 1
fi

if [ ! -f "$SCRIPT_DIR/config.jsonc" ]; then
    echo "ERROR: config.jsonc not found at $SCRIPT_DIR/config.jsonc"
    echo "Please ensure config.jsonc is in the same directory as this script."
    exit 1
fi

SUDO=""
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
fi

$SUDO cp "$SERVICE_BIN" /usr/local/bin/universal-search-service
$SUDO cp "$SCRIPT_DIR/config.jsonc" /etc/universal-search.jsonc

cat > /tmp/universal-search.service << 'EOF'
[Unit]
Description=Universal Search Service
After=network.target firecrawl.service
Wants=firecrawl.service

[Service]
Type=simple
ExecStart=/usr/local/bin/universal-search-service run
WorkingDirectory=/usr/local/bin
Restart=on-failure
RestartSec=5
Nice=19
IOSchedulingClass=idle
CPUSchedulingPolicy=idle
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF

$SUDO mv /tmp/universal-search.service /etc/systemd/system/universal-search.service
$SUDO systemctl daemon-reload
$SUDO systemctl enable universal-search.service
$SUDO systemctl start universal-search.service

sleep 2

if $SUDO systemctl is-active --quiet universal-search.service; then
    echo "Universal Search Service started successfully on port 3005"
else
    echo "Service may still be starting... check with: sudo systemctl status universal-search"
fi

echo
echo "Service installed and running."
echo
echo "To stop:   sudo systemctl stop universal-search"
echo "To start:  sudo systemctl start universal-search"
echo "To remove: sudo systemctl disable --now universal-search"
echo "Status:    sudo systemctl status universal-search"
echo "Logs:      sudo journalctl -u universal-search -f"
