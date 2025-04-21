#!/bin/bash

# Run Hyperswitch Authentication Flow Test
#
# This script provides a convenient way to run the auth_flow_test.py script
# with proper environment variables and options.

set -e

# Directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default values
TEST_TYPE="all"
VERBOSE=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --email)
      HYPERSWITCH_TEST_EMAIL="$2"
      shift 2
      ;;
    --password)
      HYPERSWITCH_TEST_PASSWORD="$2"
      shift 2
      ;;
    --totp)
      HYPERSWITCH_TEST_TOTP="$2"
      shift 2
      ;;
    --test)
      TEST_TYPE="$2"
      shift 2
      ;;
    -v|--verbose)
      VERBOSE="--verbose"
      shift
      ;;
    --help)
      echo -e "${GREEN}Hyperswitch Authentication Flow Test${NC}"
      echo
      echo "Usage: $0 [options]"
      echo
      echo "Options:"
      echo "  --email EMAIL      Email for authentication"
      echo "  --password PASS    Password for authentication"
      echo "  --totp CODE        TOTP code for 2FA"
      echo "  --test TYPE        Test type (signin, 2fa, userinfo, profile, signout, all)"
      echo "  -v, --verbose      Enable verbose output"
      echo "  --help             Show this help message"
      echo
      echo "Environment variables:"
      echo "  HYPERSWITCH_TEST_EMAIL     Email for authentication"
      echo "  HYPERSWITCH_TEST_PASSWORD  Password for authentication"
      echo "  HYPERSWITCH_TEST_TOTP      TOTP code for 2FA"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}Error: Python 3 is required but not installed.${NC}"
    exit 1
fi

# Export environment variables
export HYPERSWITCH_TEST_EMAIL
export HYPERSWITCH_TEST_PASSWORD
export HYPERSWITCH_TEST_TOTP

# Print test information
echo -e "${GREEN}Running Hyperswitch Authentication Flow Test${NC}"
echo -e "${YELLOW}Test type:${NC} $TEST_TYPE"
if [ -n "$HYPERSWITCH_TEST_EMAIL" ]; then
    echo -e "${YELLOW}Email:${NC} $HYPERSWITCH_TEST_EMAIL"
else
    echo -e "${YELLOW}Email:${NC} Using default (test@example.com)"
fi
if [ -n "$VERBOSE" ]; then
    echo -e "${YELLOW}Verbose mode:${NC} Enabled"
fi

# Run the test
echo
echo -e "${GREEN}Starting test...${NC}"
python3 auth_flow_test.py --test "$TEST_TYPE" $VERBOSE

# Check exit status
STATUS=$?
if [ $STATUS -eq 0 ]; then
    echo -e "\n${GREEN}Test completed successfully!${NC}"
else
    echo -e "\n${RED}Test failed with exit code: $STATUS${NC}"
    echo "Check auth_flow_test.log for details"
fi

exit $STATUS 