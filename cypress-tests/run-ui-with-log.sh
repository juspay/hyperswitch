#!/usr/bin/env bash
# Launch Cypress UI mode and tee all terminal output (cli_log tasks, request ids) to a log file.
cd "$(dirname "$0")"
LOG="cypress-ui.log"
: > "$LOG"
echo "Logging to $(pwd)/$LOG"
DEBUG="${DEBUG:-cypress:server:task}" npx cypress open 2>&1 | tee -a "$LOG"
