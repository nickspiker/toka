#!/bin/bash
# Toka Development Build Script
# Builds capsule, updates portal, ensures web server is running

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TOKA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WWW_DIR="$TOKA_DIR/www"
CAPSULES_DIR="$WWW_DIR/capsules"
SERVER_PORT=8000
SERVER_PID_FILE="/tmp/toka-server.pid"

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║              Toka Development Build Pipeline                ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 1: Build Toka
echo -e "${YELLOW}[1/5]${NC} Building Toka..."
cd "$TOKA_DIR"
cargo build --release --example box
echo -e "${GREEN}✓${NC} Toka built successfully"
echo ""

# Step 2: Generate capsules
echo -e "${YELLOW}[2/5]${NC} Generating capsules..."
mkdir -p "$CAPSULES_DIR"

# Run the box example to generate box.vsf
./target/release/examples/box
if [ -f "$CAPSULES_DIR/box.vsf" ]; then
    SIZE=$(stat -f%z "$CAPSULES_DIR/box.vsf" 2>/dev/null || stat -c%s "$CAPSULES_DIR/box.vsf" 2>/dev/null)
    echo -e "${GREEN}✓${NC} Generated box.vsf (${SIZE} bytes)"
else
    echo -e "${RED}✗${NC} Failed to generate box.vsf"
    exit 1
fi
echo ""

# Step 3: Verify capsule with vsfinfo (if available)
echo -e "${YELLOW}[3/5]${NC} Verifying capsule..."
if command -v vsfinfo &> /dev/null; then
    echo "Running vsfinfo on box.vsf:"
    vsfinfo "$CAPSULES_DIR/box.vsf" || {
        echo -e "${RED}⚠${NC}  vsfinfo failed - capsule may have parsing issues"
        echo "    To fix: rebuild vsfinfo with spirix feature"
        echo "    cd /mnt/Octopus/Code/vsf"
        echo "    cargo build --release --bin vsfinfo --features \"text,spirix\""
        echo "    cp target/release/vsfinfo ~/bin/"
    }
else
    echo -e "${YELLOW}⚠${NC}  vsfinfo not found - skipping capsule verification"
    echo "    Install with: cargo build --release --bin vsfinfo --features \"text,spirix\""
fi
echo ""

# Step 4: Check/start web server
echo -e "${YELLOW}[4/5]${NC} Managing web server..."

# Check if server is already running
SERVER_RUNNING=false
if [ -f "$SERVER_PID_FILE" ]; then
    PID=$(cat "$SERVER_PID_FILE")
    if ps -p $PID > /dev/null 2>&1; then
        SERVER_RUNNING=true
        echo -e "${GREEN}✓${NC} Web server already running (PID: $PID)"
    else
        # PID file exists but process is dead
        rm "$SERVER_PID_FILE"
    fi
fi

if [ "$SERVER_RUNNING" = false ]; then
    # Check if basic-http-server is installed
    if ! command -v basic-http-server &> /dev/null; then
        echo "Installing basic-http-server..."
        cargo install basic-http-server
    fi

    # Start server in background
    cd "$WWW_DIR"
    echo "Starting web server on http://localhost:$SERVER_PORT"
    nohup basic-http-server . -a 127.0.0.1:$SERVER_PORT -x > /tmp/toka-server.log 2>&1 &
    echo $! > "$SERVER_PID_FILE"
    sleep 1  # Give server time to start

    if ps -p $(cat "$SERVER_PID_FILE") > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Web server started (PID: $(cat "$SERVER_PID_FILE"))"
    else
        echo -e "${RED}✗${NC} Failed to start web server"
        cat /tmp/toka-server.log
        exit 1
    fi
fi
echo ""

# Step 5: Summary
echo -e "${YELLOW}[5/5]${NC} Build complete!"
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                     Ready to Test                            ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}Web Server:${NC}  http://localhost:$SERVER_PORT"
echo -e "${GREEN}Capsule:${NC}     $CAPSULES_DIR/box.vsf"
echo ""
echo "Commands:"
echo "  • Open browser:     xdg-open http://localhost:$SERVER_PORT (Linux)"
echo "  • Open browser:     open http://localhost:$SERVER_PORT (macOS)"
echo "  • View logs:        tail -f /tmp/toka-server.log"
echo "  • Stop server:      kill \$(cat $SERVER_PID_FILE) && rm $SERVER_PID_FILE"
echo "  • Rebuild:          ./build.sh"
echo ""
