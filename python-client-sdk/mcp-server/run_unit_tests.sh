#!/bin/bash

# Unit Tests Runner for Authentication and User Management
# This script runs the unit tests for auth.py and user.py modules

# Colors for better readability
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Print header
echo
echo -e "${CYAN}====================================================================${NC}"
echo -e "${CYAN}      AUTHENTICATION & USER MANAGEMENT UNIT TESTS RUNNER            ${NC}"
echo -e "${CYAN}====================================================================${NC}"
echo

# Set Python path to include the parent directories
echo -e "${BLUE}Setting up test environment...${NC}"
export PYTHONPATH=$(pwd)/../..

# Run the unit tests
echo -e "${BLUE}Running unit tests for auth.py and user.py modules...${NC}"
python -m unittest hyperswitch_mcp.test_auth_user_modules

# Check the test result
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed successfully!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. See details above.${NC}"
    exit 1
fi 