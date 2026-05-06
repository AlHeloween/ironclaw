#!/bin/bash
set -e

echo "============================================"
echo "  Installing Firecrawl Service (Linux)"
echo "============================================"
echo

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FC_DIR="$SCRIPT_DIR/firecrawl/apps/api"

if [ ! -d "$FC_DIR" ]; then
    echo "ERROR: Firecrawl not found at $FC_DIR"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo "ERROR: node not found. Install with: sudo pacman -S nodejs"
    exit 1
fi

if ! command -v pnpm &> /dev/null; then
    echo "ERROR: pnpm not found. Install with: sudo pacman -S pnpm"
    exit 1
fi

cd "$FC_DIR"

echo "Installing Firecrawl dependencies..."
pnpm install --frozen-lockfile 2>/dev/null || pnpm install

echo "Building Firecrawl..."
pnpm build

SUDO=""
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
fi

cat > /tmp/firecrawl.service << 'EOF'
[Unit]
Description=Firecrawl Service
After=network.target postgresql.service redis.service
Wants=postgresql.service redis.service

[Service]
Type=simple
WorkingDirectory=/opt/firecrawl/apps/api
ExecStart=/usr/bin/node dist/src/index.js
Restart=on-failure
RestartSec=5
Nice=19
IOSchedulingClass=idle
CPUSchedulingPolicy=idle
Environment="USE_GO_MARKDOWN_PARSER=false"
Environment="SKIP_DOCKER_SERVICES=true"
Environment="NUQ_RABBITMQ_URL="
Environment="NODE_NO_WARNINGS=1"
Environment="PORT=3002"
Environment="HOST=0.0.0.0"
Environment="NUQ_DATABASE_URL=postgresql://postgres:1412@localhost:5432/nuq"
Environment="NUQ_DATABASE_URL_LISTEN=postgresql://postgres:1412@localhost:5432/nuq"
Environment="REDIS_URL=redis://localhost:6379"
Environment="USE_DB_AUTHENTICATION=false"
Environment="BULL_AUTH_KEY=test"

[Install]
WantedBy=multi-user.target
EOF

$SUDO mkdir -p /opt/firecrawl
$SUDO cp -r "$FC_DIR/../../.."/* /opt/firecrawl/ 2>/dev/null || true
$SUDO mv /tmp/firecrawl.service /etc/systemd/system/firecrawl.service
$SUDO systemctl daemon-reload
$SUDO systemctl enable firecrawl.service
$SUDO systemctl start firecrawl.service

sleep 2

if $SUDO systemctl is-active --quiet firecrawl.service; then
    echo "Firecrawl service started successfully on port 3002"
else
    echo "Service may still be starting... check with: sudo systemctl status firecrawl"
fi

echo
echo "To stop:   sudo systemctl stop firecrawl"
echo "To start:  sudo systemctl start firecrawl"
echo "To remove: sudo systemctl disable --now firecrawl"
echo "Status:    sudo systemctl status firecrawl"
echo "Logs:      sudo journalctl -u firecrawl -f"
