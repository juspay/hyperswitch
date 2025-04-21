#!/bin/bash

# Authentication Flow Test Runner
# This script runs the comprehensive test for the Hyperswitch authentication flow

# Colors for better readability
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Print header
echo
echo -e "${CYAN}====================================================================${NC}"
echo -e "${CYAN}           HYPERSWITCH AUTHENTICATION FLOW TEST RUNNER              ${NC}"
echo -e "${CYAN}====================================================================${NC}"
echo

# Check if colorama is installed, install if needed
echo -e "${BLUE}Checking dependencies...${NC}"
python -c "import colorama" 2>/dev/null
if [ $? -ne 0 ]; then
    echo -e "${BLUE}Installing colorama for better test output...${NC}"
    pip install colorama
fi

# Prompt for credentials if not provided
if [ "$#" -lt 2 ]; then
    echo -e "${BLUE}Please enter your credentials:${NC}"
    read -p "Email: " EMAIL
    read -sp "Password: " PASSWORD
    echo
else
    EMAIL=$1
    PASSWORD=$2
fi

# Validate email (basic validation)
if [[ ! $EMAIL =~ ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
    echo -e "${RED}Error: Invalid email format.${NC}"
    exit 1
fi

# Run the test script
echo -e "${BLUE}Running authentication flow test...${NC}"
PYTHONPATH=$(pwd)/../.. python -m hyperswitch_mcp.test_auth_flow "$EMAIL" "$PASSWORD"

# Exit with the same status code as the test script
exit $? 