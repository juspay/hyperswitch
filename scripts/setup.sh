#!/usr/bin/env bash
set -euo pipefail

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Global cleanup function to handle error conditions and graceful exit
cleanup() {
    # Restore strict error checking
    set -e
    # Remove any temporary files if needed
    # Add any necessary cleanup operations here
    
    # The exit status passed to the function
    exit $1
}

# Set up trap to call cleanup function on script exit or interruptions
trap 'cleanup $?' EXIT
trap 'cleanup 1' INT TERM

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
    if command -v docker &> /dev/null; then
        CONTAINER_ENGINE="docker"
        echo_success "Docker is installed."
        echo ""
    elif command -v podman &> /dev/null; then
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
    if $CONTAINER_ENGINE compose version &> /dev/null; then
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

check_prerequisites() {
    # Check curl
    if ! command -v curl &> /dev/null; then
        echo_error "curl is not installed. Please install curl to proceed."
        echo ""
        exit 1
    fi
    echo_success "curl is installed."
    echo ""
    
    # Check ports
    required_ports=(8080 9000 9050 5432 6379 9060)
    unavailable_ports=()
    
    for port in "${required_ports[@]}"; do
        if command -v nc &> /dev/null; then
            if nc -z localhost "$port" 2>/dev/null; then
                unavailable_ports+=("$port")
            fi
        elif command -v lsof &> /dev/null; then
            if lsof -i :"$port" &> /dev/null; then
                unavailable_ports+=("$port")
            fi
        else
            echo_warning "Neither nc nor lsof available to check ports. Skipping port check."
            echo ""
            break
        fi
    done
    
    if [ ${#unavailable_ports[@]} -ne 0 ]; then
        echo_warning "The following ports are already in use: ${unavailable_ports[*]}"
        echo_warning "This might cause conflicts with Hyperswitch services."
        echo ""
        echo -n "Do you want to continue anyway? (y/n): "
        read -n 1 -r REPLY
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo ""
    fi
}

setup_config() {
    if [ ! -f "config/docker_compose.toml" ]; then
        echo_error "Configuration file 'config/docker_compose.toml' not found. Please ensure the file exists and is correctly configured."
        exit 1
    fi
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

start_services() {
    
    case $PROFILE in
        standalone)
            $DOCKER_COMPOSE up -d pg redis-standalone migration_runner hyperswitch-server
            ;;
        standard)
            $DOCKER_COMPOSE up -d
            ;;
        full)
            $DOCKER_COMPOSE --profile scheduler --profile monitoring --profile olap --profile full_setup up -d
            ;;
    esac
}

check_services_health() {
    # Wait for the hyperswitch-server to be healthy
    MAX_RETRIES=30
    RETRY_INTERVAL=5
    RETRIES=0
    
    while [ $RETRIES -lt $MAX_RETRIES ]; do
        response=$(curl -s -w "\\nStatus_Code:%{http_code}" http://localhost:8080/health)
        status_code=$(echo "$response" | grep "Status_Code:" | cut -d':' -f2)
        response_body=$(echo "$response" | head -n1)
        
        if [ "$status_code" = "200" ] && [ "$response_body" = "health is good" ]; then
            print_access_info
            return
        fi
        
        RETRIES=$((RETRIES+1))
        if [ $RETRIES -eq $MAX_RETRIES ]; then
            printf "\n"
            echo_error "${RED}${BOLD}Hyperswitch server did not become healthy in the expected time."
            printf "Check logs with: $DOCKER_COMPOSE logs hyperswitch-server, Or reach out to us on slack(https://hyperswitch-io.slack.com/) for help.\n"
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
    
    BASE_URL="http://localhost:8080"
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

        local headers=(-H "Content-Type: application/json" -H "api-key: hyperswitch" -H "User-Agent: HyperSwitch-Shell-Client/1.0" -H "Referer: http://localhost:9000/")

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
            curl_cmd=(curl -s -X "$method" "${headers[@]}" "$BASE_URL$endpoint")
        else
            curl_cmd=(curl -s -X "$method" "${headers[@]}" -d "$data" "$BASE_URL$endpoint")
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

            i=$((i+1))
        done
        return 1
    }

    # Test the health endpoint first to ensure the API is responsive
    health_response=$(curl -s -w "\\nStatus_Code:%{http_code}" "$BASE_URL/health")
    health_status_code=$(echo "$health_response" | grep "Status_Code:" | cut -d':' -f2)
    health_response_body=$(echo "$health_response" | head -n1)

    # Try signin first
    signin_payload="{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}"
    signin_response=$(make_api_call "POST" "/signin" "$signin_payload")
    status_code=$?
    
    # Check if user needs to be created
    if [[ $status_code -ne 0 || $(echo "$signin_response" | grep -q "error"; echo $?) -eq 0 ]]; then
        # User doesn't exist or login failed, create new account
        signup_payload="{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"country\":\"IN\"}"

        # Only try signing up once - using exact headers from browser
        # For making signup request without verbose logging
        signup_cmd="curl -s -X POST '$BASE_URL/user/signup' \
          -H 'Accept: */*' \
          -H 'Accept-Language: en-GB,en-US;q=0.9,en;q=0.8' \
          -H 'Content-Type: application/json' \
          -H 'Origin: http://localhost:9000' \
          -H 'Referer: http://localhost:9000/' \
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
            terminate_response=$(curl -s -X GET -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer $token" "$BASE_URL/user/2fa/terminate?skip_two_factor_auth=true")

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
        user_info_cmd="curl -s -X GET -H 'Content-Type: application/json' -H 'api-key: hyperswitch' -H 'authorization: Bearer $token' '$BASE_URL/user'"
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
        merchant_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer $token" -d "$merchant_payload" "$BASE_URL/accounts/$merchant_id")
        
        # Configure connector
        connector_payload=$(cat <<EOF
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
        connector_response=$(curl -s -X POST -H "Content-Type: application/json" -H "api-key: hyperswitch" -H "authorization: Bearer $token" -d "$connector_payload" "$BASE_URL/account/$merchant_id/connectors")
        
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
        printf "  • ${GREEN}${BOLD}Control Center${NC}: ${BLUE}${BOLD}http://localhost:9000${NC}\n"
        configure_account || true
    fi
    
    printf "  • ${GREEN}${BOLD}App Server${NC}: ${BLUE}${BOLD}http://localhost:8080${NC}\n"
    
    if [ "$PROFILE" = "full" ]; then
        printf "  • ${GREEN}${BOLD}Monitoring (Grafana)${NC}: ${BLUE}${BOLD}http://localhost:3000${NC}\n"
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

# Main execution flow
show_banner
detect_docker_compose
check_prerequisites
setup_config
select_profile
start_services
check_services_health  # This will call print_access_info if the server is healthy
