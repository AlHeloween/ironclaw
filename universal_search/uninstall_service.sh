#!/bin/bash
set -e

echo "============================================"
echo "  Removing Universal Search Services (Linux)"
echo "============================================"
echo
echo "This will remove:"
echo "  - Universal Search Service (universal-search)"
echo "  - Firecrawl Service (firecrawl)"
echo
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled."
    exit 0
fi

SUDO=""
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
fi

echo
echo "Stopping services..."
$SUDO systemctl stop universal-search 2>/dev/null || true
$SUDO systemctl stop firecrawl 2>/dev/null || true

echo "Disabling services..."
$SUDO systemctl disable universal-search 2>/dev/null || true
$SUDO systemctl disable firecrawl 2>/dev/null || true

echo "Removing service files..."
$SUDO rm -f /etc/systemd/system/universal-search.service
$SUDO rm -f /etc/systemd/system/firecrawl.service
$SUDO rm -f /usr/local/bin/universal-search-service
$SUDO rm -f /etc/universal-search.jsonc
$SUDO rm -rf /opt/firecrawl

$SUDO systemctl daemon-reload

echo
echo "Services removed successfully."
