#!/bin/bash
# Stop Toka development server

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SERVER_PID_FILE="/tmp/toka-server.pid"

if [ -f "$SERVER_PID_FILE" ]; then
    PID=$(cat "$SERVER_PID_FILE")
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${YELLOW}Stopping Toka web server (PID: $PID)...${NC}"
        kill $PID
        rm "$SERVER_PID_FILE"
        echo -e "${GREEN}✓${NC} Server stopped"
    else
        echo -e "${YELLOW}⚠${NC}  Server not running (stale PID file)"
        rm "$SERVER_PID_FILE"
    fi
else
    echo -e "${YELLOW}⚠${NC}  No server PID file found - server may not be running"

    # Try to find and kill any basic-http-server processes on port 8000
    PID=$(lsof -t -i:8000 2>/dev/null)
    if [ -n "$PID" ]; then
        echo -e "${YELLOW}Found process on port 8000 (PID: $PID), stopping...${NC}"
        kill $PID
        echo -e "${GREEN}✓${NC} Process stopped"
    fi
fi
