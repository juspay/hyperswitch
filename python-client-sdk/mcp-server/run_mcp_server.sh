#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status.

# Get the absolute path of the script's directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "[run_mcp_server.sh] Changed directory to: $SCRIPT_DIR"

# Activate virtual environment (assuming .venv exists here)
if [ -d ".venv/bin" ]; then
    echo "[run_mcp_server.sh] Activating virtual environment..."
    source .venv/bin/activate
else
    echo "[run_mcp_server.sh] WARNING: .venv/bin/activate not found. Skipping activation."
fi

# Set PYTHONPATH to include the parent directory (python-client-sdk)
export PYTHONPATH="$SCRIPT_DIR/.."
echo "[run_mcp_server.sh] Set PYTHONPATH to: $PYTHONPATH"

# Run the MCP server using python -m
echo "[run_mcp_server.sh] Running server: python -m hyperswitch_mcp.server"
python -m hyperswitch_mcp.server 