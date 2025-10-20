#!/bin/bash
set -euo pipefail

# Playwright Test Execution Script
# Supports both Stripe and Cybersource connectors with multi-tab parallel execution

# Configuration
CONNECTOR=${1:-stripe}
HEADLESS=${HEADLESS:-true}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
print_header() {
  echo -e "${BLUE}========================================${NC}"
  echo -e "${BLUE}$1${NC}"
  echo -e "${BLUE}========================================${NC}"
}

print_success() {
  echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
  echo -e "${RED}✗ $1${NC}"
}

print_info() {
  echo -e "${YELLOW}ℹ $1${NC}"
}

# Main execution
print_header "Playwright E2E Tests - $CONNECTOR"

# Validate environment
if [ -z "${PLAYWRIGHT_BASEURL:-}" ]; then
  print_info "PLAYWRIGHT_BASEURL not set, using default: http://localhost:8080"
  export PLAYWRIGHT_BASEURL="http://localhost:8080"
fi

if [ -z "${PLAYWRIGHT_ADMINAPIKEY:-}" ]; then
  print_error "PLAYWRIGHT_ADMINAPIKEY is required!"
  exit 1
fi

export PLAYWRIGHT_CONNECTOR="$CONNECTOR"

# Display configuration
print_info "Connector: $CONNECTOR"
print_info "Base URL: $PLAYWRIGHT_BASEURL"
print_info "Headless: $HEADLESS"
print_info "Workers: $(grep -c ^processor /proc/cpuinfo 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 2)"

# Run tests
print_header "Phase 1: Sequential Setup Tests (0000-0003)"
if npx playwright test \
  --project=1-core-setup \
  --project=2-account-setup \
  --project=3-customer-setup \
  --project=4-connector-setup; then
  print_success "Setup tests completed successfully"
else
  print_error "Setup tests failed!"
  exit 1
fi

echo ""
print_header "Phase 2: Parallel Tests (Multi-Tab Execution)"
if npx playwright test --project=${CONNECTOR}-parallel; then
  print_success "Parallel tests completed successfully"
else
  print_error "Parallel tests failed!"
  exit 1
fi

echo ""
print_success "All tests completed successfully for $CONNECTOR!"

# Generate and show report
if command -v open >/dev/null 2>&1 || command -v xdg-open >/dev/null 2>&1; then
  print_info "Generating HTML report..."
  npx playwright show-report --host 127.0.0.1 &
  print_success "Report server started"
fi

exit 0
