#!/bin/bash
# Hyperswitch Business Profiles API Test Script
# This script tests the various endpoints for Business Profiles API

# Default values
EMAIL="jarnura47@gmail.com"
PASSWORD="Qwerty@123"
API_KEY="hypwerswitch"
JWT_TOKEN=""
ACCOUNT_ID="merchant_1745015734"
BASE_URL="http://localhost:8080"
TEST="all"
VERBOSE=true  # Set verbose to true by default for debugging
PROFILE_ID=""

# Sample business profile for testing
SAMPLE_PROFILE='{
  "profile_name": "Test Profile API Test",
  "return_url": "https://example.com/return",
  "enable_payment_response_hash": true,
  "redirect_to_merchant_with_http_post": false,
  "webhook_details": {
    "webhook_url": "https://example.com/webhook",
    "webhook_version": "v1"
  },
  "metadata": {
    "description": "Test business profile created by automated test",
    "business_name": "Test Business",
    "registration_number": "REG123456"
  },
  "use_billing_as_payment_method_billing": true,
  "session_expiry": 900
}'

# Function to print usage
usage() {
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  --email EMAIL             Email address for authentication (default: $EMAIL)"
  echo "  --password PASSWORD       Password for authentication (default: $PASSWORD)"
  echo "  --token TOKEN             JWT token (if already obtained)"
  echo "  --api_key API_KEY         API key for Hyperswitch API (default: $API_KEY)"
  echo "  --account_id ACCOUNT_ID   Account ID (default: $ACCOUNT_ID)"
  echo "  --base_url URL            Base URL for API (default: $BASE_URL)"
  echo "  --test TEST               Test to run (list|create|get|update|all) (default: all)"
  echo "  --profile_id PROFILE_ID   Profile ID for get/update operations"
  echo "  --verbose                 Enable verbose output"
  echo "  --help                    Show this help message"
  exit 1
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --email)
      EMAIL="$2"
      shift 2
      ;;
    --password)
      PASSWORD="$2"
      shift 2
      ;;
    --token)
      JWT_TOKEN="$2"
      shift 2
      ;;
    --api_key)
      API_KEY="$2"
      shift 2
      ;;
    --account_id)
      ACCOUNT_ID="$2"
      shift 2
      ;;
    --base_url)
      BASE_URL="$2"
      shift 2
      ;;
    --test)
      TEST="$2"
      shift 2
      ;;
    --profile_id)
      PROFILE_ID="$2"
      shift 2
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --help)
      usage
      ;;
    *)
      echo "Unknown option: $1"
      usage
      ;;
  esac
done

# Function to log messages
log() {
  echo "[$(date +"%Y-%m-%d %H:%M:%S")] $1"
}

# Function to log verbose messages
log_verbose() {
  if [ "$VERBOSE" = true ]; then
    log "$1"
  fi
}

# Get JWT token if not provided
if [ -z "$JWT_TOKEN" ]; then
  log "Getting JWT token..."
  
  # Check if auth_script.py exists
  if [ -f "jwt_api_docs/auth_script.py" ]; then
    # Get a clean token without any logging in it
    JWT_TOKEN=$(python jwt_api_docs/auth_script.py --email "$EMAIL" --password "$PASSWORD" --token-only)
    
    # Save token to file for debugging if needed
    echo "$JWT_TOKEN" > jwt_api_docs/jwt_token.txt
  else
    # Try to read from token file
    if [ -f "jwt_api_docs/jwt_token.txt" ]; then
      JWT_TOKEN=$(cat jwt_api_docs/jwt_token.txt)
      log "Read token from jwt_api_docs/jwt_token.txt"
    else
      log "ERROR: Could not get JWT token. Please provide --token or ensure auth_script.py exists."
      exit 1
    fi
  fi
  
  if [ -z "$JWT_TOKEN" ]; then
    log "ERROR: Failed to get JWT token. Check credentials and try again."
    exit 1
  fi
  
  log "JWT Token acquired: ${JWT_TOKEN:0:15}...${JWT_TOKEN: -15}"
fi

# Function to validate endpoint URL
validate_endpoint() {
  local endpoint=$1
  log "DEBUG: Testing endpoint: $endpoint"
  
  # Test if the API server is reachable
  HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health")
  log "DEBUG: Health check status: $HTTP_STATUS"
  
  if [ "$HTTP_STATUS" != "200" ]; then
    log "WARNING: API server health check failed with status $HTTP_STATUS"
  fi
}

# Test List Business Profiles without API key
test_without_api_key() {
  log "Testing List Business Profiles API WITHOUT API key..."
  
  # Check if the endpoint exists first
  ENDPOINT="$BASE_URL/account/$ACCOUNT_ID/business_profile"
  validate_endpoint "$ENDPOINT"
  
  log "DEBUG: Using endpoint: $ENDPOINT"
  log "DEBUG: JWT Token: ${JWT_TOKEN:0:15}...${JWT_TOKEN: -15}"
  log "DEBUG: Not sending API Key"
  
  # Use -v flag to see request/response details
  HTTP_CODE=$(curl -v -s -o response_no_key.txt -w "%{http_code}" \
    -H "Authorization: Bearer $JWT_TOKEN" \
    "$ENDPOINT" 2>curl_error_no_key.log)
  
  log "DEBUG: HTTP Status Code: $HTTP_CODE"
  
  # Print curl debug output
  log "DEBUG: Curl verbose output:"
  cat curl_error_no_key.log
  
  # Print raw response
  log "Raw Response:"
  cat response_no_key.txt
  log "End of Raw Response"
  
  RESPONSE=$(cat response_no_key.txt)
  
  # Try to pretty print, but don't fail if it's not valid JSON
  log "Response (formatted if possible):"
  echo "$RESPONSE" | python -m json.tool || echo "Could not parse response as JSON"
  
  # Check for common error patterns
  if [[ "$HTTP_CODE" != "2"* ]]; then
    log "ERROR: API returned non-success status code: $HTTP_CODE"
  fi
  
  # Cleanup
  rm -f response_no_key.txt curl_error_no_key.log
}

# Test List Business Profiles
test_list_profiles() {
  log "Testing List Business Profiles API..."
  
  # Check if the endpoint exists first
  ENDPOINT="$BASE_URL/account/$ACCOUNT_ID/business_profile"
  validate_endpoint "$ENDPOINT"
  
  log "DEBUG: Using endpoint: $ENDPOINT"
  log "DEBUG: JWT Token: ${JWT_TOKEN:0:15}...${JWT_TOKEN: -15}"
  log "DEBUG: API Key: $API_KEY"
  
  # Use -v flag to see request/response details
  HTTP_CODE=$(curl -v -s -o response.txt -w "%{http_code}" \
    -H "Authorization: Bearer $JWT_TOKEN" \
    -H "api-key: $API_KEY" \
    "$ENDPOINT" 2>curl_error.log)
  
  log "DEBUG: HTTP Status Code: $HTTP_CODE"
  
  # Print curl debug output
  log "DEBUG: Curl verbose output:"
  cat curl_error.log
  
  # Print raw response
  log "Raw Response:"
  cat response.txt
  log "End of Raw Response"
  
  RESPONSE=$(cat response.txt)
  
  # Try to pretty print, but don't fail if it's not valid JSON
  log "Response (formatted if possible):"
  echo "$RESPONSE" | python -m json.tool || echo "Could not parse response as JSON"
  
  # Check for common error patterns
  if [[ "$HTTP_CODE" != "2"* ]]; then
    log "ERROR: API returned non-success status code: $HTTP_CODE"
  fi
  
  if [[ "$RESPONSE" == *"\"profiles\""* ]]; then
    log "SUCCESS: Retrieved business profiles"
    
    # Extract profile IDs for later use
    PROFILE_IDS=$(echo "$RESPONSE" | python -c "
import json, sys
try:
    data = json.load(sys.stdin)
    if 'profiles' in data and len(data['profiles']) > 0:
        for profile in data['profiles']:
            if 'profile_id' in profile:
                print(profile['profile_id'])
                break
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null)
    
    if [ ! -z "$PROFILE_IDS" ]; then
      PROFILE_ID="$PROFILE_IDS"
      log "Found profile ID: $PROFILE_ID"
      echo "$PROFILE_ID" > jwt_api_docs/profile_id.txt
      log "Saved profile ID to jwt_api_docs/profile_id.txt"
    fi
  else
    log "WARNING: No profiles or error in response"
  fi
  
  # Cleanup
  rm -f response.txt
}

# Test Create Business Profile
test_create_profile() {
  log "Testing Create Business Profile API..."
  
  # Create a temporary file with the profile JSON
  PROFILE_FILE=$(mktemp)
  echo "$SAMPLE_PROFILE" > "$PROFILE_FILE"
  
  log_verbose "Request body:"
  log_verbose "$SAMPLE_PROFILE"
  
  # Check if the endpoint exists first
  ENDPOINT="$BASE_URL/account/$ACCOUNT_ID/business_profile"
  validate_endpoint "$ENDPOINT"
  
  log "DEBUG: Using endpoint: $ENDPOINT"
  log "DEBUG: JWT Token: ${JWT_TOKEN:0:15}...${JWT_TOKEN: -15}"
  log "DEBUG: API Key: $API_KEY"
  
  # Use -v flag to see request/response details
  HTTP_CODE=$(curl -s -o response.txt -w "%{http_code}" \
    -X POST \
    -H "Authorization: Bearer $JWT_TOKEN" \
    -H "api-key: $API_KEY" \
    -H "Content-Type: application/json" \
    -d @"$PROFILE_FILE" \
    "$ENDPOINT")
  
  log "DEBUG: HTTP Status Code: $HTTP_CODE"
  
  # Print raw response
  log "Raw Response:"
  cat response.txt
  log "End of Raw Response"
  
  RESPONSE=$(cat response.txt)
  
  # Cleanup
  rm "$PROFILE_FILE"
  
  # Try to pretty print, but don't fail if it's not valid JSON
  log "Response (formatted if possible):"
  echo "$RESPONSE" | python -m json.tool || echo "Could not parse response as JSON"
  
  # Check for common error patterns
  if [[ "$HTTP_CODE" != "2"* ]]; then
    log "ERROR: API returned non-success status code: $HTTP_CODE"
  fi
  
  # Check if the response contains a profile_id
  if [[ "$RESPONSE" == *"\"profile_id\""* ]]; then
    PROFILE_ID=$(echo "$RESPONSE" | python -c "
import json, sys
try:
    data = json.load(sys.stdin)
    if 'profile_id' in data:
        print(data['profile_id'])
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
" 2>/dev/null)
    
    if [ ! -z "$PROFILE_ID" ]; then
      log "SUCCESS: Created business profile with ID: $PROFILE_ID"
      echo "$PROFILE_ID" > jwt_api_docs/profile_id.txt
      log "Saved profile ID to jwt_api_docs/profile_id.txt"
    else
      log "WARNING: Could not extract profile ID from response"
    fi
  else
    log "ERROR: Failed to create business profile"
  fi
  
  # Cleanup
  rm -f response.txt
}

# Test Get Business Profile
test_get_profile() {
  # If profile_id is not provided, try to read from file
  if [ -z "$PROFILE_ID" ]; then
    if [ -f "jwt_api_docs/profile_id.txt" ]; then
      PROFILE_ID=$(cat jwt_api_docs/profile_id.txt)
      log "Read profile ID from jwt_api_docs/profile_id.txt: $PROFILE_ID"
    else
      log "ERROR: Profile ID not provided and not found in jwt_api_docs/profile_id.txt"
      log "Please run list or create test first, or provide --profile_id"
      return 1
    fi
  fi
  
  log "Testing Get Business Profile API for ID: $PROFILE_ID"
  
  # Check if the endpoint exists first
  ENDPOINT="$BASE_URL/account/$ACCOUNT_ID/business_profile/$PROFILE_ID"
  validate_endpoint "$ENDPOINT"
  
  log "DEBUG: Using endpoint: $ENDPOINT"
  
  # Use -v flag to see request/response details
  HTTP_CODE=$(curl -s -o response.txt -w "%{http_code}" \
    -H "Authorization: Bearer $JWT_TOKEN" \
    -H "api-key: $API_KEY" \
    "$ENDPOINT")
  
  log "DEBUG: HTTP Status Code: $HTTP_CODE"
  
  # Print raw response
  log "Raw Response:"
  cat response.txt
  log "End of Raw Response"
  
  RESPONSE=$(cat response.txt)
  
  # Try to pretty print, but don't fail if it's not valid JSON
  log "Response (formatted if possible):"
  echo "$RESPONSE" | python -m json.tool || echo "Could not parse response as JSON"
  
  # Check for common error patterns
  if [[ "$HTTP_CODE" != "2"* ]]; then
    log "ERROR: API returned non-success status code: $HTTP_CODE"
  fi
  
  # Check if the response contains the profile
  if [[ "$RESPONSE" == *"\"profile_id\""* && "$RESPONSE" == *"\"$PROFILE_ID\""* ]]; then
    log "SUCCESS: Retrieved business profile"
  else
    log "ERROR: Failed to retrieve business profile"
  fi
  
  # Cleanup
  rm -f response.txt
}

# Test Update Business Profile
test_update_profile() {
  # If profile_id is not provided, try to read from file
  if [ -z "$PROFILE_ID" ]; then
    if [ -f "jwt_api_docs/profile_id.txt" ]; then
      PROFILE_ID=$(cat jwt_api_docs/profile_id.txt)
      log "Read profile ID from jwt_api_docs/profile_id.txt: $PROFILE_ID"
    else
      log "ERROR: Profile ID not provided and not found in jwt_api_docs/profile_id.txt"
      log "Please run list or create test first, or provide --profile_id"
      return 1
    fi
  fi
  
  log "Testing Update Business Profile API for ID: $PROFILE_ID"
  
  # Create updated profile with timestamp
  TIMESTAMP=$(date +"%Y%m%d%H%M%S")
  UPDATE_PROFILE=$(echo "$SAMPLE_PROFILE" | python -c "
import json, sys
try:
    data = json.load(sys.stdin)
    data['profile_name'] = f'Updated Profile {\"$TIMESTAMP\"}'
    if 'metadata' not in data:
        data['metadata'] = {}
    data['metadata']['update_note'] = f'Updated by test script at {\"$TIMESTAMP\"}'
    print(json.dumps(data))
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)
")
  
  # Create a temporary file with the updated profile JSON
  PROFILE_FILE=$(mktemp)
  echo "$UPDATE_PROFILE" > "$PROFILE_FILE"
  
  log_verbose "Request body:"
  log_verbose "$UPDATE_PROFILE"
  
  # Check if the endpoint exists first
  ENDPOINT="$BASE_URL/account/$ACCOUNT_ID/business_profile/$PROFILE_ID"
  validate_endpoint "$ENDPOINT"
  
  log "DEBUG: Using endpoint: $ENDPOINT"
  
  # Use -v flag to see request/response details
  HTTP_CODE=$(curl -v -s -o response.txt -w "%{http_code}" \
    -X POST \
    -H "Authorization: Bearer $JWT_TOKEN" \
    -H "api-key: $API_KEY" \
    -H "Content-Type: application/json" \
    -d @"$PROFILE_FILE" \
    "$ENDPOINT" 2>curl_error.log)
  
  log "DEBUG: HTTP Status Code: $HTTP_CODE"
  
  # Print curl debug output
  log "DEBUG: Curl verbose output:"
  cat curl_error.log
  
  # Print raw response
  log "Raw Response:"
  cat response.txt
  log "End of Raw Response"
  
  RESPONSE=$(cat response.txt)
  
  rm "$PROFILE_FILE"
  
  # Try to pretty print, but don't fail if it's not valid JSON
  log "Response (formatted if possible):"
  echo "$RESPONSE" | python -m json.tool || echo "Could not parse response as JSON"
  
  # Check for common error patterns
  if [[ "$HTTP_CODE" != "2"* ]]; then
    log "ERROR: API returned non-success status code: $HTTP_CODE"
  fi
  
  # Check if the response indicates success
  if [[ "$RESPONSE" == *"\"profile_id\""* && "$RESPONSE" == *"\"$PROFILE_ID\""* ]]; then
    log "SUCCESS: Updated business profile"
  else
    log "ERROR: Failed to update business profile"
  fi
  
  # Cleanup
  rm -f response.txt curl_error.log
}

# Test with alternate URL formats
test_alternate_url() {
  log "Testing List Business Profiles API with alternate URL format..."
  
  # Try alternative endpoint formats
  ENDPOINTS=(
    "$BASE_URL/account/$ACCOUNT_ID/business_profiles"
  )
  
  for ENDPOINT in "${ENDPOINTS[@]}"; do
    log "DEBUG: Trying endpoint: $ENDPOINT"
    
    # Use -v flag to see request/response details
    HTTP_CODE=$(curl -v -s -o response_alt.txt -w "%{http_code}" \
      -H "Authorization: Bearer $JWT_TOKEN" \
      -H "api-key: $API_KEY" \
      "$ENDPOINT" 2>curl_error_alt.log)
    
    log "DEBUG: HTTP Status Code: $HTTP_CODE"
    
    # Print curl debug output
    log "DEBUG: Curl verbose output:"
    cat curl_error_alt.log
    
    # Print raw response
    log "Raw Response:"
    cat response_alt.txt
    log "End of Raw Response"
    
    # If we got a successful response, save it
    if [[ "$HTTP_CODE" == "2"* ]]; then
      log "SUCCESS: Found working endpoint: $ENDPOINT"
      echo "$ENDPOINT" > jwt_api_docs/working_endpoint.txt
      break
    fi
  done
  
  # Cleanup
  rm -f response_alt.txt curl_error_alt.log
}

# Test that API server is running
log "Checking if API server is running..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health")
log "API server health check status: $HTTP_STATUS"

if [ "$HTTP_STATUS" != "200" ]; then
  log "WARNING: API server may not be running or is not responding to health checks"
  log "Please ensure the Hyperswitch server is running at $BASE_URL"
else
  log "API server is running"
fi

# Run tests based on the TEST parameter
case $TEST in
  "list")
    test_without_api_key
    test_list_profiles
    test_alternate_url
    ;;
  "create")
    test_create_profile
    ;;
  "get")
    test_get_profile
    ;;
  "update")
    test_update_profile
    ;;
  "all")
    log "Running all tests sequentially..."
    test_without_api_key
    test_list_profiles
    test_create_profile
    test_get_profile
    test_update_profile
    ;;
  *)
    log "ERROR: Unknown test type: $TEST"
    usage
    ;;
esac

log "Test script completed!"
exit 0 