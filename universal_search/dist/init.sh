#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SERVICE_NAME=universal-search
LOG_DIR="$SCRIPT_DIR/logs"

echo "============================================"
echo " Universal Search Service - Initialization"
echo "============================================"
echo ""

mkdir -p "$LOG_DIR"

echo "Step 1/4: Checking prerequisites..."
MISSING=""
command -v git &>/dev/null || MISSING="$MISSING git"
command -v node &>/dev/null || MISSING="$MISSING node"
if ! command -v pnpm &>/dev/null; then
    echo "  Installing pnpm..."
    npm install -g pnpm
fi
if [ -n "$MISSING" ]; then
    echo "  Missing:$MISSING"
    echo "  Please install and re-run."
    exit 1
fi
echo "  Prerequisites OK"

echo ""
echo "Step 2/4: Initializing Firecrawl..."
if [ -d "$SCRIPT_DIR/firecrawl/.git" ]; then
    echo "  Firecrawl already present"
else
    echo "  Cloning Firecrawl..."
    git clone --depth=1 https://github.com/firecrawl/firecrawl.git "$SCRIPT_DIR/firecrawl"
fi
echo "  Installing Firecrawl dependencies..."
(cd "$SCRIPT_DIR/firecrawl/apps/api" && pnpm install)

echo ""
echo "Step 3/4: Creating configuration..."
if [ -f "$SCRIPT_DIR/config.jsonc" ]; then
    echo "  Config already exists"
elif [ -f "$SCRIPT_DIR/config.jsonc.template" ]; then
    cp "$SCRIPT_DIR/config.jsonc.template" "$SCRIPT_DIR/config.jsonc"
fi

echo ""
echo "Step 4/4: Installing and starting service..."

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    UNIT_FILE="$HOME/.config/systemd/user/${SERVICE_NAME}.service"
    mkdir -p "$HOME/.config/systemd/user"

    if [ -f "$UNIT_FILE" ] && systemctl --user is-active "$SERVICE_NAME" &>/dev/null; then
        echo "  Service already running"
    else
        cat > "$UNIT_FILE" << EOF
[Unit]
Description=Universal Search Service
After=network.target

[Service]
Type=simple
ExecStart=$SCRIPT_DIR/universal-search-service run --config $SCRIPT_DIR/config.jsonc
Restart=always
RestartSec=3
WorkingDirectory=$SCRIPT_DIR
StandardOutput=append:$LOG_DIR/universal-search.log
StandardError=append:$LOG_DIR/universal-search.log

[Install]
WantedBy=default.target
EOF
        systemctl --user daemon-reload
        loginctl enable-linger
        systemctl --user enable "$SERVICE_NAME"
        systemctl --user start "$SERVICE_NAME"
        echo "  systemd service installed and started"
    fi

elif [[ "$OSTYPE" == "darwin"* ]]; then
    PLIST_FILE="$HOME/Library/LaunchAgents/com.universalsearch.daemon.plist"
    mkdir -p "$HOME/Library/LaunchAgents"

    if [ -f "$PLIST_FILE" ] && launchctl list | grep -q "com.universalsearch.daemon"; then
        echo "  Service already running"
    else
        cat > "$PLIST_FILE" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.universalsearch.daemon</string>
  <key>ProgramArguments</key>
  <array>
    <string>$SCRIPT_DIR/universal-search-service</string>
    <string>run</string>
    <string>--config</string>
    <string>$SCRIPT_DIR/config.jsonc</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>WorkingDirectory</key>
  <string>$SCRIPT_DIR</string>
  <key>StandardOutPath</key>
  <string>$LOG_DIR/universal-search.log</string>
  <key>StandardErrorPath</key>
  <string>$LOG_DIR/universal-search.log</string>
</dict>
</plist>
EOF
        launchctl load -w "$PLIST_FILE"
        launchctl start com.universalsearch.daemon
        echo "  LaunchAgent installed and started"
    fi
else
    echo "  Unsupported OS, running in foreground..."
    "$SCRIPT_DIR/universal-search-service" run --config "$SCRIPT_DIR/config.jsonc"
fi

echo ""
echo "============================================"
echo " Initialization complete!"
echo "  Service: $SERVICE_NAME"
echo "  Logs:    $LOG_DIR"
echo "============================================"
echo ""
echo "  Verifying service health..."
sleep 5
if command -v curl &>/dev/null; then
    if curl -sf http://localhost:3005/health &>/dev/null; then
        echo "  Service is healthy on port 3005"
    else
        echo "  Warning: Service not responding on port 3005"
    fi
else
    if timeout 5 bash -c "cat < /dev/null > /dev/tcp/localhost/3005" 2>/dev/null; then
        echo "  Service is healthy on port 3005"
    else
        echo "  Warning: Service not responding on port 3005"
    fi
fi
