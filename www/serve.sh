#!/bin/bash
# Simple HTTP server for testing Toka WASM in browser

echo "Toka VM Browser Test Server"
echo "=============================="
echo ""
echo "Open browser to: http://localhost:8000"
echo "Press Ctrl+C to stop"
echo ""

# Try python3 first, then python, then node
if command -v python3 &> /dev/null; then
    python3 -m http.server 8000
elif command -v python &> /dev/null; then
    python -m http.server 8000
elif command -v npx &> /dev/null; then
    npx http-server -p 8000
else
    echo "Error: No HTTP server available"
    echo "Install Python or Node.js to run this script"
    exit 1
fi
