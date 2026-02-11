#!/bin/bash
# Quick development cycle: rebuild and restart
# This is a convenience wrapper that stops, builds, and optionally opens browser

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Toka Development Cycle${NC}"
echo ""

# Stop existing server
if [ -f "$SCRIPT_DIR/stop.sh" ]; then
    "$SCRIPT_DIR/stop.sh"
    echo ""
fi

# Build and start
"$SCRIPT_DIR/build.sh"

# Optional: auto-open browser
if [ "$1" = "--open" ] || [ "$1" = "-o" ]; then
    echo ""
    echo "Opening browser..."
    if command -v xdg-open &> /dev/null; then
        xdg-open http://localhost:8000
    elif command -v open &> /dev/null; then
        open http://localhost:8000
    else
        echo "Could not detect browser command (xdg-open or open)"
    fi
fi
