#! /usr/bin/env bash
set -Eeuo pipefail
# Set traps for errors and interruptions
trap 'handle_error "$LINENO" "$?"' ERR
trap 'handle_interrupt' INT TERM

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Variables for installation status
VERSION="unknown"
INSTALLATION_STATUS="initiated"
SCARF_PARAMS=()

# Function to print colorful messages
echo_info() {
    printf "${BLUE}[INFO]${NC} %s\n" "$1"
}

echo_success() {
    printf "${GREEN}[SUCCESS]${NC} %s\n" "$1"
}

echo_warning() {
    printf "${YELLOW}[WARNING]${NC} %s\n" "$1"
}

echo_error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
}

show_banner() {
    printf "${BLUE}${BOLD}\n"
    printf "\n"
    printf "        #             \n"
    printf "        # #    #  ####  #####    ##   #   #  \n"
    printf "        # #    # #      #    #  #  #   # #   \n"
    printf "        # #    #  ####  #    # #    #   #    \n"
    printf "  #     # #    #      # #####  ######   #    \n"
    printf "  #     # #    # #    # #      #    #   #    \n"
    printf "   #####   ####   ####  #      #    #   #    \n"
    printf "\n"
    printf "\n"
    printf "  #     # #   # #####  ###### #####   ####  #    # # #####  ####  #    # \n"
    printf "  #     #  # #  #    # #      #    # #      #    # #   #   #    # #    # \n"
    printf "  #     #   #   #    # #####  #    #  ####  #    # #   #   #      ###### \n"
    printf "  #######   #   #####  #      #####       # # ## # #   #   #      #    # \n"
    printf "  #     #   #   #      #      #   #  #    # ##  ## #   #   #    # #    # \n"
    printf "  #     #   #   #      ###### #    #  ####  #    # #   #    ####  #    # \n"
    printf "\n"
    sleep 1
    printf "${NC}\n"
    printf "🚀 ${BLUE}One-Click Docker Setup${NC} 🚀\n"
}

# Detect Docker Compose version
detect_docker_compose() {
    # Check Docker or Podman
    if command -v docker &>/dev/null; then
        CONTAINER_ENGINE="docker"
        echo_success "Docker is installed."
        echo ""
    elif command -v podman &>/dev/null; then
        CONTAINER_ENGINE="podman"
        echo_success "Podman is installed."
        echo ""
    else
        echo_error "Neither Docker nor Podman is installed. Please install one of them to proceed."
        echo_info "Visit https://docs.docker.com/get-docker/ or https://podman.io/docs/installation for installation instructions."
        echo_info "After installation, re-run this script: scripts/setup.sh"
        echo ""
        exit 1
    fi

    # Check Docker Compose or Podman Compose
    if $CONTAINER_ENGINE compose version &>/dev/null; then
        DOCKER_COMPOSE="$CONTAINER_ENGINE compose"
        echo_success "Compose is installed for $CONTAINER_ENGINE."
        echo ""
    else
        echo_error "Compose is not installed for $CONTAINER_ENGINE. Please install $CONTAINER_ENGINE compose to proceed."
        echo ""
        if [ "$CONTAINER_ENGINE" = "docker" ]; then
            echo_info "Visit https://docs.docker.com/compose/install/ for installation instructions."
            echo ""
        elif [ "$CONTAINER_ENGINE" = "podman" ]; then
            echo_info "Visit https://podman-desktop.io/docs/compose/setting-up-compose for installation instructions."
            echo ""
        fi
        echo_info "After installation, re-run this script: scripts/setup.sh"
        echo ""
        exit 1
    fi
}

# Function to check if a port is in use (returns 0 if in use, 1 if available)
check_port_availability() {
    local port=$1
    if command -v nc &>/dev/null; then
        if nc -z localhost "${port}" 2>/dev/null; then
            return 0 # Port is in use
        else
            return 1 # Port is available
        fi
    elif command -v lsof &>/dev/null; then
        if lsof -i :"${port}" &>/dev/null; then
            return 0 # Port is in use
        else
            return 1 # Port is available
        fi
    else
        tools_available=false
        return 1 # Unable to check, assume available
    fi
}

# Function to find the next available port starting from a given port
find_next_available_port() {
    local port=$1
    local next_port=$port

    while check_port_availability "$next_port"; do
        next_port=$((next_port + 1))
        # Add an upper bound to prevent infinite loop
        if [[ $next_port -gt $((port + 1000)) ]]; then
            echo_error "Could not find an available port in range $port-$((port + 1000))"
            return $port # Return the original port if no available port found
        fi
    done

    echo "$next_port"
}

# Function to write port configuration to .service-ports.env file
write_env_file() {
    local mode=$1
    local env_file=".service-ports.env"

    echo "# Service ports configuration" >"$env_file"
    echo "# Generated on $(date)" >>"$env_file"
    echo "" >>"$env_file"

    if [[ "$mode" == "original" ]]; then
        # Write original ports
        for service_port in "${services_with_ports[@]}"; do
            service="${service_port%%:*}"
            port="${service_port##*:}"
            # Convert service name to uppercase without using ${var^^} syntax
            service_upper=$(echo "$service" | tr '[:lower:]' '[:upper:]')
            echo "${service_upper}_PORT=$port" >>"$env_file"
        done
        echo_info "Default ports written to $env_file"
    elif [[ "$mode" == "new" ]]; then
        # Write new ports - all ports are in the new_service_ports array now
        for i in "${!services_with_ports[@]}"; do
            service="${services_with_ports[$i]%%:*}"
            # Convert service name to uppercase without using ${var^^} syntax
            service_upper=$(echo "$service" | tr '[:lower:]' '[:upper:]')
            echo "${service_upper}_PORT=${new_service_ports[$i]}" >>"$env_file"
        done
        echo_info "New ports configuration written to $env_file"
    else
        echo_error "Invalid mode specified for writing env file"
        return 1
    fi

    return 0
}

check_prerequisites() {
    # Check curl
    if ! command -v curl &>/dev/null; then
        echo_error "curl is not installed. Please install curl to proceed."
        echo ""
        exit 1
    fi
    echo_success "curl is installed."
    echo ""

    # Define required services and ports
    services_with_ports=("hyperswitch_server:8080" "hyperswitch_control_center:9000" "hyperswitch_web:9050" "postgres:5432" "redis:6379" "unified_checkout:9060")
    unavailable_ports=()
    services_with_unavailable_ports=()
    new_service_ports=()
    new_available_ports=()
    tools_available=true

    # Check each port and collect unavailable ones
    for service_port in "${services_with_ports[@]}"; do
        service="${service_port%%:*}"
        port="${service_port##*:}"

        if [ "$tools_available" = false ]; then
            break
        fi
        if check_port_availability "$port"; then
            unavailable_ports+=("$port")
            services_with_unavailable_ports+=("$service_port")

            # Find next available port
            next_port=$(find_next_available_port "$port")
            new_available_ports+=("$next_port")
            new_service_ports+=("$next_port")
        else
            new_service_ports+=("${port}") # Empty string for available ports
        fi
    done

    # Report the results
    if [ ${#unavailable_ports[@]} -ne 0 ]; then
        echo ""
        echo_warning "The following ports are already in use:"
        for i in "${!unavailable_ports[@]}"; do
            service="${services_with_unavailable_ports[$i]%%:*}"
            port="${unavailable_ports[$i]}"
            next="${new_available_ports[$i]}"
            echo " - $service: Port $port is in use, next available: $next"
        done
        echo ""

        # Present options to the user
        echo "Please choose one of the following options:"
        echo ""
        echo -e "1) ${YELLOW}Use the next available ports${NC} : ${BLUE}[Default]${NC}"
        echo "   This will automatically select alternative free ports for services"
        echo ""
        echo -e "2) ${YELLOW}Override existing ports${NC} :"
        echo "   This will attempt to use the default ports even though they appear to be in use"
        echo -e "   ${YELLOW}[Warning]${NC}: This may cause conflicts with existing services"
        echo ""
        echo -e "3) ${YELLOW}Exit the process${NC} :"
        echo "   This will terminate the setup without making any changes"
        echo ""
        echo -n "Enter your choice (1-3): "
        read -n 1 user_choice
        user_choice=${user_choice:-1}
        echo

        case $user_choice in
        1)
            echo_success "Using next available ports."
            write_env_file "new"
            ;;

        2)
            echo_warning "You chose to override the ports. This might cause conflicts."
            write_env_file "original"
            ;;
        3)
            echo_warning "Exiting the process."
            exit 0
            ;;
        *)
            echo_error "Invalid choice. Exiting."
            exit 1
            ;;
        esac
    else
        if [ "$tools_available" = false ]; then
            echo_warning "Neither nc nor lsof available to check ports. Assuming ports are available."
        else
            echo_success "All required ports are available."
        fi
        echo ""
        # Write the original ports to the env file since all are available
        write_env_file "original"
    fi
}

setup_config() {
    if [ ! -f "config/docker_compose.toml" ]; then
        echo_error "Configuration file 'config/docker_compose.toml' not found. Please ensure the file exists and is correctly configured."
        exit 1
    fi
    HYPERSWITCH_BASE_URL="http://localhost:${HYPERSWITCH_SERVER_PORT:-8080}"
}

select_profile() {
    printf "\n"
    printf "Select a setup option:\n"
    printf "1) ${YELLOW}Standard Setup${NC}: ${BLUE}[Recommended]${NC} Ideal for quick trial.\n"
    printf "   Services included: ${BLUE}App Server, Control Center, PostgreSQL and Redis${NC}\n"
    printf "\n"
    printf "2) ${YELLOW}Full Stack Setup${NC}: Ideal for comprehensive end-to-end payment testing.\n"
    printf "   Services included: ${BLUE}Everything in Standard, Monitoring and Scheduler${NC}\n"
    printf "\n"
    printf "3) ${YELLOW}Standalone App Server${NC}: Ideal for API-first integration testing.\n"
    printf "   Services included: ${BLUE}App Server, PostgreSQL and Redis)${NC}\n"
    echo ""
    local profile_selected=false
    while [ "$profile_selected" = false ]; do
        echo -n "Enter your choice (1-3): "
        read -n 1 profile_choice
        echo

        case $profile_choice in
        1)
            PROFILE="standard"
            profile_selected=true
            ;;
        2)
            PROFILE="full"
            profile_selected=true
            ;;
        3)
            PROFILE="standalone"
            profile_selected=true
            ;;
        *)
            echo_error "Invalid choice. Please enter 1, 2, or 3."
            ;;
        esac
    done

    echo "Selected setup: $PROFILE"
}

scarf_call() {
    # Call the Scarf webhook endpoint with the provided parameters
    chmod +x scripts/notify_scarf.sh
    if [ $INSTALLATION_STATUS = "initiated" ]; then
        scripts/notify_scarf.sh "version=${VERSION}" "status=${INSTALLATION_STATUS}" >/dev/null 2>&1
    else
        scripts/notify_scarf.sh "version=${VERSION}" "status=${INSTALLATION_STATUS}" "${SCARF_PARAMS[@]}" >/dev/null 2>&1
    fi
    # Reset SCARF_PARAMS for the next call
    SCARF_PARAMS=()
}

start_services() {

    case $PROFILE in
    standalone)
        $DOCKER_COMPOSE --env-file .service-ports.env up -d pg redis-standalone migration_runner hyperswitch-server
        ;;
    standard)
        $DOCKER_COMPOSE --env-file .service-ports.env up -d
        ;;
    full)
        $DOCKER_COMPOSE --env-file .service-ports.env --profile scheduler --profile monitoring --profile olap --profile full_setup up -d
        ;;
    esac
}

deep_health_check() {
    HYPERSWITCH_DEEP_HEALTH_URL="${HYPERSWITCH_BASE_URL}/health/ready"
    VERSION=$(curl --silent --output /dev/null --request GET --write-out '%header{x-hyperswitch-version}' "${HYPERSWITCH_DEEP_HEALTH_URL}" | sed 's/-dirty$//')
    HEALTH_RESPONSE=$(curl --silent "${HYPERSWITCH_DEEP_HEALTH_URL}")

    if [[ "$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error')" != 'null' ]]; then
        INSTALLATION_STATUS="error"
        ERROR_TYPE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.type')
        ERROR_MESSAGE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.message')
        ERROR_CODE=$(echo "${HEALTH_RESPONSE}" | jq --raw-output '.error.code')
        SCARF_PARAMS+=(
            "error_type='${ERROR_TYPE}'"
            "error_message='${ERROR_MESSAGE}'"
            "error_code='${ERROR_CODE}'"
        )
    else
        INSTALLATION_STATUS="success"
        for key in $(echo "${HEALTH_RESPONSE}" | jq --raw-output 'keys_unsorted[]'); do
            value=$(echo "${HEALTH_RESPONSE}" | jq --raw-output --arg key "${key}" '.[$key]')
            SCARF_PARAMS+=("${key}=${value}")
        done
    fi
    # Call the Scarf webhook endpoint with the provided parameters
    scarf_call
}

check_services_health() {
    # Wait for the hyperswitch-server to be healthy
    MAX_RETRIES=30
    RETRY_INTERVAL=5
    RETRIES=0

    while [ $RETRIES -lt $MAX_RETRIES ]; do
        response=$(curl -s -w "\\nStatus_Code:%{http_code}" "${HYPERSWITCH_BASE_URL}/health")
        status_code=$(echo "$response" | grep "Status_Code:" | cut -d':' -f2)
        response_body=$(echo "$response" | head -n1)

        if [ "$status_code" = "200" ] && [ "$response_body" = "health is good" ]; then
            deep_health_check
            print_access_info
            return
        fi

        RETRIES=$((RETRIES + 1))
        if [ $RETRIES -eq $MAX_RETRIES ]; then
            INSTALLATION_STATUS="error"
            ERROR_TYPE="timeout"
            ERROR_MESSAGE="Hyperswitch server did not become healthy in the expected time."
            ERROR_CODE="503"
            SCARF_PARAMS+=(
                "error_type='${ERROR_TYPE}'"
                "error_message='${ERROR_MESSAGE}'"
                "error_code='${ERROR_CODE}'"
            )
            scarf_call

            printf "\n"
            echo_error "${RED}${BOLD}${ERROR_MESSAGE}"
            printf "Check logs with: $DOCKER_COMPOSE logs hyperswitch-server, Or reach out to us on slack(https://hyperswitch-io.slack.com/) for help."
            printf "The setup process will continue, but some services might not work correctly.${NC}"
            printf "\n"
        else
            printf "Waiting for server to become healthy... (%d/%d)\n" $RETRIES $MAX_RETRIES
            sleep $RETRY_INTERVAL
        fi
    done
}

configure_account() {
    # Temporarily disable strict error checking to prevent premature exit
    set +e
    local show_credentials_flag=false

    EMAIL="demo@hyperswitch.com"
    PASSWORD="Hyperswitch@123"
    # Initialize merchant_id and profile_id to empty strings
    merchant_id=""
    profile_id=""

    # Function to make API calls with proper headers
    make_api_call() {
        local method=$1
        local endpoint=$2
        local data=$3
        local auth_header=${4:-}

        # Ensure endpoint starts with /user if it doesn't already
        if [[ ! $endpoint =~ ^/user && ! $endpoint =~ ^/health && ! $endpoint =~ ^/accounts && ! $endpoint =~ ^/account ]]; then
            endpoint="/user$endpoint"
        fi

        local headers=(-H "Content-Type: application/json" -H "api-key: hyperswitch" -H "User-Agent: HyperSwitch-Shell-Client/1.0" -H "Referer: http://localhost:${HYPERSWITCH_CONTROL_CENTER_PORT:-9000}/")

        if [ -n "$auth_header" ]; then
            headers+=(-H "authorization: Bearer $auth_header")
        fi

        if [ -n "$merchant_id" ]; then
            headers+=(-H "X-Merchant-Id: $merchant_id")
        fi

        if [ -n "$profile_id" ]; then
            headers+=(-H "X-Profile-Id: $profile_id")
        fi

        local curl_cmd
        if [ "$method" = "GET" ]; then
            curl_cmd=(curl -s -X "$method" "${headers[@]}" "$HYPERSWITCH_BASE_URL$endpoint")
        else
            curl_cmd=(curl -s -X "$method" "${headers[@]}" -d "$data" "$HYPERSWITCH_BASE_URL$endpoint")
        fi

        local retries=3
        local i=0
        while [ $i -lt $retries ]; do
            response=$("${curl_cmd[@]}")
            local response_code=$("${curl_cmd[@]}" -o /dev/null -s -w "%{http_code}")

            if [ $response_code -lt 400 ]; then
                echo "$response"
                return 0
            fi

            i=$((i + 1))
        done
        return 1
    }

    # Test the health endpoint first to ensure the API is responsive
    health_response=$(curl -s -w "\\nStatus_Code:%{http_code}" "$HYPERSWITCH_BASE_URL/health")
    health_status_code=$(echo "$health_response" | grep "Status_Code:" | cut -d':' -f2)
    health_response_body=$(echo "$health_response" | head -n1)

    # Try signin first
    signin_payload="{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}"
    signin_response=$(make_api_call "POST" "/signin" "$signin_payload")
    status_code=$?

    # Check if user needs to be created
    if [[ $status_code -ne 0 || $(
        echo "$signin_response" | grep -q "error"
        echo $?
    ) -eq 0 ]]; then
        # User doesn't exist or login failed, create new account
        signup_payload="{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"country\":\"IN\"}"

        # Only try signing up once - using exact headers from browser
        # For making signup request without verbose logging
        signup_cmd="curl -s -X POST '$HYPERSWITCH_BASE_URL/user/signup' \
          -H 'Accept: */*' \
          -H 'Accept-Language: en-GB,en-US;q=0.9,en;q=0.8' \
          -H 'Content-Type: application/json' \
          -H 'Origin: http://localhost:${HYPERSWITCH_CONTROL_CENTER_PORT:-9000}' \
          -H 'Referer: http://localhost:${HYPERSWITCH_CONTROL_CENTER_PORT:-9000}/' \
          -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36' \
          -H 'api-key: hyperswitch' \
          -d '$signup_payload'"

        signup_response=$(eval "$signup_cmd")

        # Extract token from signup response
        token=$(echo "$signup_response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        token_type=$(echo "$signup_response" | grep -o '"token_type":"[^"]*"' | cut -d'"' -f4)

        if [ -n "$token" ]; then
            show_credentials_flag=true
        fi
        is_new_user=true
    else
        auth_response="$signin_response"
        token=$(echo "$auth_response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        token_type=$(echo "$auth_response" | grep -o '"token_type":"[^"]*"' | cut -d'"' -f4)
        if [ -n "$token" ]; then
            show_credentials_flag=true
        fi
        is_new_user=false
    fi

    # Handle 2FA if needed
    if [ "$token_type" = "totp" ]; then
        MAX_RETRIES=3
        for i in $(seq 1 $MAX_RETRIES); do
            terminate_response=$(curl -s -X GET -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer $token" "$HYPERSWITCH_BASE_URL/user/2fa/terminate?skip_two_factor_auth=true")

            new_token=$(echo "$terminate_response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
            if [ -n "$new_token" ]; then
                token="$new_token"
                break
            else
                if [ $i -lt $MAX_RETRIES ]; then
                    sleep 1
                fi
            fi
        done
    fi

    # Get user info
    if [ -n "$token" ]; then
        user_info_cmd="curl -s -X GET -H 'Content-Type: application/json' -H 'api-key: hyperswitch' -H 'authorization: Bearer $token' '$HYPERSWITCH_BASE_URL/user'"
        user_info=$(eval "$user_info_cmd")
    else
        user_info="{}"
    fi

    merchant_id=$(echo "$user_info" | grep -o '"merchant_id":"[^"]*"' | cut -d'"' -f4 || echo "")
    profile_id=$(echo "$user_info" | grep -o '"profile_id":"[^"]*"' | cut -d'"' -f4 || echo "")

    # Configure account for new users
    if [ "$is_new_user" = true ] && [ -n "$merchant_id" ] && [ -n "$token" ]; then
        # Create merchant account
        merchant_payload="{\"merchant_id\":\"$merchant_id\",\"merchant_name\":\"Test\"}"
        merchant_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer $token" -d "$merchant_payload" "$HYPERSWITCH_BASE_URL/accounts/$merchant_id")

        # Configure connector
        connector_payload=$(
            cat <<EOF
{
    "connector_type": "payment_processor",
    "profile_id": "$profile_id",
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
        connector_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer $token" -d "$connector_payload" "$HYPERSWITCH_BASE_URL/account/$merchant_id/connectors")

        # Silently check if configuration was successful without printing messages
        if [ -z "$(echo "$merchant_response" | grep -o 'merchant_id')" ] || [ -z "$(echo "$connector_response" | grep -o 'connector_id')" ]; then
            # Only log to debug log if we want to troubleshoot later
            : # No-op command
        fi
    fi

    # Provide helpful information to the user regardless of success/failure
    if [ "$show_credentials_flag" = true ]; then
        printf "            Use the following credentials:\n"
        printf "            Email:    $EMAIL\n"
        printf "            Password: $PASSWORD\n"
    fi

    # Restore strict error checking
    set -e
}

print_access_info() {
    printf "${BLUE}"
    printf "╔════════════════════════════════════════════════════════════════╗\n"
    printf "║             Welcome to Juspay Hyperswitch!                     ║\n"
    printf "╚════════════════════════════════════════════════════════════════╝\n"
    printf "${NC}\n"

    printf "${GREEN}${BOLD}Setup complete! You can now access Hyperswitch services at:${NC}\n"

    if [ "$PROFILE" != "standalone" ]; then
        printf "  • ${GREEN}${BOLD}Control Center${NC}: ${BLUE}${BOLD}${HYPERSWITCH_CONTROL_CENTER_PORT:-9000}${NC}\n"
        configure_account || true
    fi

    printf "  • ${GREEN}${BOLD}App Server${NC}: ${BLUE}${BOLD}${HYPERSWITCH_BASE_URL}${NC}\n"

    if [ "$PROFILE" = "full" ]; then
        printf "  • ${GREEN}${BOLD}Monitoring (Grafana)${NC}: ${BLUE}${BOLD}${UNIFIED_CHECKOUT_PORT:-9060}${NC}\n"
    fi
    printf "\n"

    # Provide the stop command based on the selected profile
    echo_info "To stop all services, run the following command:"
    case $PROFILE in
    standalone)
        printf "${BLUE}$DOCKER_COMPOSE down${NC}\n"
        ;;
    standard)
        printf "${BLUE}$DOCKER_COMPOSE down${NC}\n"
        ;;
    full)
        printf "${BLUE}$DOCKER_COMPOSE --profile scheduler --profile monitoring --profile olap --profile full_setup down${NC}\n"
        ;;
    esac
    printf "\n"
    printf "Reach out to us on ${BLUE}https://hyperswitch-io.slack.com${NC} in case you face any issues.\n"
}

handle_error() {
    local lineno=$1
    local exit_code=$2
    local last_command="${BASH_COMMAND:-unknown}"

    # Set global vars used by scarf_call
    INSTALLATION_STATUS="error"
    ERROR_MESSAGE="Command '\$ ${last_command}' failed at line ${lineno} with exit code ${exit_code}"

    SCARF_PARAMS+=(
        "error_type=bash_error"
        "error_message=${ERROR_MESSAGE}"
        "error_code=${exit_code}"
    )

    scarf_call
    cleanup
    exit $exit_code
}

cleanup() {
    set -e
    # Clean up any started containers if we've selected a profile
    if [ -n "${PROFILE:-}" ]; then
        echo_info "Cleaning up any started containers..."
        case $PROFILE in
        standalone)
            $DOCKER_COMPOSE down >/dev/null 2>&1 || true
            ;;
        standard)
            $DOCKER_COMPOSE down >/dev/null 2>&1 || true
            ;;
        full)
            $DOCKER_COMPOSE --profile scheduler --profile monitoring --profile olap --profile full_setup down >/dev/null 2>&1 || true
            ;;
        esac
    fi
}

# Handle user interruptions
handle_interrupt() {
    echo ""
    echo_warning "Script interrupted by user"
    # Set appropriate error information for user abort
    INSTALLATION_STATUS="user_aborted"
    SCARF_PARAMS=(
        "error_type=user_interrupt"
        "error_message=Script interrupted by user"
        "error_code=130"
    )

    # Call scarf to report the interruption
    scarf_call
    cleanup
    exit 130
}

# Main execution flow
scarf_call
show_banner
detect_docker_compose
check_prerequisites
source .service-ports.env
setup_config
select_profile
start_services
check_services_health # This will call print_access_info if the server is healthy
