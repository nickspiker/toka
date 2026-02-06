#!/bin/bash
# Simple HTTP server for testing Toka WASM in browser

echo "Toka VM Browser Test Server"
echo "=============================="
echo "Open browser to: http://localhost:8000"
echo "Press Ctrl+C to stop"
echo ""

# Check if basic-http-server is installed
if ! command -v basic-http-server &> /dev/null; then
    echo "Installing basic-http-server..."
    cargo install basic-http-server
    echo ""
fi

# Serve current directory on localhost:8000
basic-http-server . -a 127.0.0.1:8000
