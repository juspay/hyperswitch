#!/bin/bash
# Script to get JWT token and test connectivity

# Get the email and password from command line or use defaults
EMAIL=${1:-jarnura47@gmail.com}
PASSWORD=${2:-"Qwerty@123"}

# HARDCODED VALUES (temporary fix)
ACCOUNT_ID="merchant_1745015734"
API_KEY="hypwerswitch"  # We'll need to find this from the dashboard

# Get the JWT token using the --token-only flag
JWT_TOKEN=$(python jwt_api_docs/auth_script.py --email "$EMAIL" --password "$PASSWORD" --token-only)

# Check if we got a token
if [ -z "$JWT_TOKEN" ]; then
    echo "Failed to get JWT token. Check credentials and try again."
    exit 1
fi

# Print the token (first and last 15 chars)
echo "JWT Token: ${JWT_TOKEN:0:15}...${JWT_TOKEN: -15}"

# Save the token to a file for easier use
echo "$JWT_TOKEN" > jwt_api_docs/jwt_token.txt
echo "Token saved to jwt_api_docs/jwt_token.txt"

# Save the hardcoded account ID
echo "$ACCOUNT_ID" > jwt_api_docs/account_id.txt
echo -e "\nUsing hardcoded Account ID: $ACCOUNT_ID"
echo "Account ID saved to jwt_api_docs/account_id.txt"

# Output instructions for running tests
echo -e "\nNow you need to get your API key from the Hyperswitch dashboard"
echo "Once you have the API key, you can run the Business Profiles tests with:"
echo -e "\n./jwt_api_docs/run_business_profiles_test.sh \\"
echo "  --email $EMAIL \\"
echo "  --password \"$PASSWORD\" \\"
echo "  --api_key $API_KEY \\"
echo "  --account_id $ACCOUNT_ID \\"
echo "  --test list"

# Try a simple GET request to see if we can list profiles without an API key
echo -e "\nTrying to list business profiles without API key:"
PROFILES_RESPONSE=$(curl -s -H "Authorization: Bearer $JWT_TOKEN" "http://localhost:8080/account/$ACCOUNT_ID/business_profile")

# Print the raw response
echo "Response from profiles endpoint:"
echo "$PROFILES_RESPONSE"

exit 0 