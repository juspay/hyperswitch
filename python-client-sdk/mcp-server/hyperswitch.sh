#!/bin/bash
set -e  # Exit on error

# Get the absolute path of the script's directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Create and activate virtual environment if it doesn't exist
if [ ! -d ".venv" ]; then
    ~/.local/bin/uv venv .venv
fi

# Activate virtual environment
source .venv/bin/activate

# Install required package if not already installed
~/.local/bin/uv pip install mcp

# Verify mcp installation
~/.local/bin/uv pip show mcp

# Set ADMIN_API_KEY and run the server
export ADMIN_API_KEY="test_admin"
PYTHONPATH="$SCRIPT_DIR/.." ~/.local/bin/uv run hyperswitch/server.py 