#!/bin/bash

# Master Test Runner
# This script runs all tests for the authentication and user management APIs

# Colors for better readability
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Set Python path to include the parent directories
export PYTHONPATH=$(pwd)/../..

# Print header
echo
echo -e "${CYAN}====================================================================${NC}"
echo -e "${CYAN}      HYPERSWITCH AUTHENTICATION & USER MANAGEMENT TEST SUITE       ${NC}"
echo -e "${CYAN}====================================================================${NC}"
echo

# Global variables to track test status
unit_tests_status=0
auth_flow_status=0
tests_failed=0

# Function to print section header
print_section() {
    echo
    echo -e "${BLUE}====================================================================${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}====================================================================${NC}"
    echo
}

# Run unit tests
run_unit_tests() {
    print_section "RUNNING UNIT TESTS"
    
    echo -e "${BLUE}Testing auth.py and user.py modules...${NC}"
    python -m unittest hyperswitch_mcp.test_auth_user_modules
    
    unit_tests_status=$?
    if [ $unit_tests_status -eq 0 ]; then
        echo -e "${GREEN}✓ Unit tests passed!${NC}"
    else
        echo -e "${RED}✗ Unit tests failed.${NC}"
        ((tests_failed++))
    fi
}

# Run authentication flow test
run_auth_flow_test() {
    print_section "RUNNING AUTHENTICATION FLOW TEST"
    
    # Check if credentials were provided
    if [ "$#" -lt 2 ]; then
        echo -e "${YELLOW}⚠ Skipping authentication flow test - no credentials provided.${NC}"
        echo -e "${YELLOW}  To run this test, provide email and password:${NC}"
        echo -e "${YELLOW}  ./run_all_tests.sh <email> <password>${NC}"
        return 0
    fi
    
    echo -e "${BLUE}Testing complete authentication flow...${NC}"
    python -m hyperswitch_mcp.test_auth_flow "$1" "$2"
    
    auth_flow_status=$?
    if [ $auth_flow_status -eq 0 ]; then
        echo -e "${GREEN}✓ Authentication flow test passed!${NC}"
    else
        echo -e "${RED}✗ Authentication flow test failed.${NC}"
        ((tests_failed++))
    fi
}

# Print test summary
print_summary() {
    print_section "TEST SUMMARY"
    
    echo -e "Unit Tests: $([ $unit_tests_status -eq 0 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    
    if [ "$#" -lt 2 ]; then
        echo -e "Authentication Flow Test: ${YELLOW}SKIPPED${NC}"
    else
        echo -e "Authentication Flow Test: $([ $auth_flow_status -eq 0 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    fi
    
    echo
    if [ $tests_failed -eq 0 ]; then
        if [ "$#" -lt 2 ]; then
            echo -e "${YELLOW}⚠ Unit tests passed, but authentication flow test was skipped.${NC}"
        else
            echo -e "${GREEN}✓ All tests passed successfully!${NC}"
        fi
    else
        echo -e "${RED}✗ $tests_failed test suite(s) failed. See details above.${NC}"
    fi
}

# Main execution
run_unit_tests
run_auth_flow_test "$@"
print_summary "$@"

# Exit with appropriate status code
if [ $tests_failed -eq 0 ]; then
    exit 0
else
    exit 1
fi 