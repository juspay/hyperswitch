#! /usr/bin/env bash
EMAIL="demo@hyperswitch.com"
PASSWORD="Hyperswitch@123"
# Initialize merchant_id and profile_id to empty strings
merchant_id=""
profile_id=""

# Test the health endpoint first to ensure the API is responsive
health_response=$(curl -s -w "\\nStatus_Code:%{http_code}" "${HYPERSWITCH_SERVER_URL}/health")
health_status_code=$(echo "${health_response}" | grep "Status_Code:" | cut -d':' -f2)
health_response_body=$(echo "${health_response}" | head -n1)

# Try signin first
signin_payload="{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"
signin_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "User-Agent: HyperSwitch-Shell-Client/1.0" -H "Referer: ${HYPERSWITCH_CONTROL_CENTER_URL}/" -d "${signin_payload}" "${HYPERSWITCH_SERVER_URL}/user/signin")

# Check if user needs to be created
if [[ $(
    echo "${signin_response}" | grep -q "error"
    echo $?
) -eq 0 ]]; then
    # User doesn't exist or login failed, create new account
    signup_payload="{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\",\"country\":\"IN\"}"

    # Only try signing up once - using exact headers from browser
    # For making signup request without verbose logging
    signup_cmd="curl -s -X POST '${HYPERSWITCH_SERVER_URL}/user/signup' \
        -H 'Accept: */*' \
        -H 'Accept-Language: en-GB,en-US;q=0.9,en;q=0.8' \
        -H 'Content-Type: application/json' \
        -H 'Origin: ${HYPERSWITCH_CONTROL_CENTER_URL}' \
        -H 'Referer: ${HYPERSWITCH_CONTROL_CENTER_URL}/' \
        -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36' \
        -H 'api-key: hyperswitch' \
        -d '${signup_payload}'"

    signup_response=$(eval "${signup_cmd}")

    # Extract token from signup response
    token=$(echo "${signup_response}" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
    token_type=$(echo "${signup_response}" | grep -o '"token_type":"[^"]*"' | cut -d'"' -f4)
    is_new_user=true
else
    auth_response="${signin_response}"
    token=$(echo "${auth_response}" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
    token_type=$(echo "${auth_response}" | grep -o '"token_type":"[^"]*"' | cut -d'"' -f4)
    is_new_user=false
fi

# Handle 2FA if needed
if [ "${token_type}" = "totp" ]; then
    MAX_RETRIES=3
    for i in $(seq 1 ${MAX_RETRIES}); do
        terminate_response=$(curl -s -X GET -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer ${token}" "${HYPERSWITCH_SERVER_URL}/user/2fa/terminate?skip_two_factor_auth=true")

        new_token=$(echo "${terminate_response}" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        if [ -n "${new_token}" ]; then
            token="${new_token}"
            break
        else
            if [ $i -lt ${MAX_RETRIES} ]; then
                sleep 1
            fi
        fi
    done
fi

# Get user info
if [ -n "${token}" ]; then
    user_info_cmd="curl -s -X GET -H 'Content-Type: application/json' -H 'api-key: hyperswitch' -H 'authorization: Bearer ${token}' '${HYPERSWITCH_SERVER_URL}/user'"
    user_info=$(eval "${user_info_cmd}")
else
    user_info="{}"
fi

merchant_id=$(echo "${user_info}" | grep -o '"merchant_id":"[^"]*"' | cut -d'"' -f4 || echo "")
profile_id=$(echo "${user_info}" | grep -o '"profile_id":"[^"]*"' | cut -d'"' -f4 || echo "")

# Configure account for new users
if [ "${is_new_user}" = true ] && [ -n "${merchant_id}" ] && [ -n "${token}" ]; then
    # Create merchant account
    merchant_payload="{\"merchant_id\":\"${merchant_id}\",\"merchant_name\":\"Test\"}"
    merchant_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer ${token}" -d "${merchant_payload}" "${HYPERSWITCH_SERVER_URL}/accounts/${merchant_id}")

    # Configure connector
    connector_payload=$(
        cat <<EOF
{
"connector_type": "payment_processor",
"profile_id": "${profile_id}",
"connector_name": "paypal_test",
"connector_label": "paypal_test_default",
"disabled": false,
"test_mode": true,
"payment_methods_enabled": [
    {
        "payment_method": "card",
        "payment_method_types": [
            {
                "payment_method_type": "debit",
                "card_networks": [
                    "Mastercard"
                ],
                "minimum_amount": 0,
                "maximum_amount": 68607706,
                "recurring_enabled": true,
                "installment_payment_enabled": false
            },
            {
                "payment_method_type": "credit",
                "card_networks": [
                    "Visa"
                ],
                "minimum_amount": 0,
                "maximum_amount": 68607706,
                "recurring_enabled": true,
                "installment_payment_enabled": false
            }
        ]
    }
],
"metadata": {},
"connector_account_details": {
    "api_key": "test_key",
    "auth_type": "HeaderKey"
},
"status": "active"
}
EOF
    )
    connector_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer ${token}" -d "${connector_payload}" "${HYPERSWITCH_SERVER_URL}/account/${merchant_id}/connectors")

    # Silently check if configuration was successful without printing messages
    if [ -z "$(echo "${merchant_response}" | grep -o 'merchant_id')" ] || [ -z "$(echo "${connector_response}" | grep -o 'connector_id')" ]; then
        # Only log to debug log if we want to troubleshoot later
        : # No-op command
    fi
fi
