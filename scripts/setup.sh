#!/usr/bin/env bash
set -Eeuo pipefail

# Set up error logging - redirect stderr to both log file and console
ERROR_LOG="error.log"
exec 2> >(tee -a "${ERROR_LOG}" >&2)

# Set traps for errors and interruptions
trap 'handle_error "$LINENO" "$BASH_COMMAND" "$?"' ERR
trap 'handle_interrupt' INT TERM

# Variables for installation status
VERSION="unknown"
INSTALLATION_STATUS="initiated"
SCARF_PARAMS=()

# Trap and handle any errors that occur during the script execution
handle_error() {
    local lineno=$1
    local last_command=$2
    local exit_code=$3

    # Capture recent error log content if available
    local log_content=""
    if [ -f "${ERROR_LOG}" ] && [ -s "${ERROR_LOG}" ]; then
        # Get last 5 lines of error log, escape for URL encoding
        log_content=$(tail -n 1 "${ERROR_LOG}" | tr '\n' '|' | sed 's/|$//')
    fi

    # Set global vars used by scarf_call
    INSTALLATION_STATUS="error"
    ERROR_MESSAGE="Command '\$ ${last_command}' failed at line ${lineno} with exit code ${exit_code} and error logs: ${log_content:-'not available'}"

    SCARF_PARAMS+=(
        "error_type=script_error"
        "error_message=${ERROR_MESSAGE}"
        "error_code=${exit_code}"
    )

    scarf_call
    cleanup
    exit $exit_code
}

# Handle user interruptions
handle_interrupt() {
    echo ""
    echo_warning "Script interrupted by user"
    # Set appropriate error information for user abort
    INSTALLATION_STATUS="user_interrupt"

    # Call scarf to report the interruption
    scarf_call
    cleanup
    exit 130
}

cleanup() {
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

    # Optionally remove error log if it's empty or on successful completion
    if [ -f "${ERROR_LOG}" ]; then
        rm -f "${ERROR_LOG}"
    fi
}

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

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
        DOCKER_COMPOSE="${CONTAINER_ENGINE} compose"
        echo_success "Compose is installed for ${CONTAINER_ENGINE}."
        echo ""
    else
        echo_error "Compose is not installed for ${CONTAINER_ENGINE}. Please install ${CONTAINER_ENGINE} compose to proceed."
        echo ""
        if [ "${CONTAINER_ENGINE}" = "docker" ]; then
            echo_info "Visit https://docs.docker.com/compose/install/ for installation instructions."
            echo ""
        elif [ "${CONTAINER_ENGINE}" = "podman" ]; then
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
    if ! command -v curl &>/dev/null; then
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
        if command -v nc &>/dev/null; then
            if nc -z localhost "$port" 2>/dev/null; then
                unavailable_ports+=("$port")
            fi
        elif command -v lsof &>/dev/null; then
            if lsof -i :"$port" &>/dev/null; then
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

    # Create an .env file for one-click setup
    local env_file=".oneclick-setup.env"
    echo "# One-Click Setup configuration" >"${env_file}"
    echo "# Generated on $(date)" >>"${env_file}"
    echo "" >>"${env_file}"

    # Enable one-click setup mode
    echo "ONE_CLICK_SETUP=true" >>"${env_file}"
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
    while [ "${profile_selected}" = "false" ]; do
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

    echo "Selected setup: ${PROFILE}"
}

scarf_call() {
    # Call the Scarf webhook endpoint with the provided parameters
    chmod +x scripts/notify_scarf.sh
    if [ ${#SCARF_PARAMS[@]} -eq 0 ]; then
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
        $DOCKER_COMPOSE --env-file .oneclick-setup.env up -d pg redis-standalone migration_runner hyperswitch-server
        ;;
    standard)
        $DOCKER_COMPOSE --env-file .oneclick-setup.env up -d
        ;;
    full)
        $DOCKER_COMPOSE --env-file .oneclick-setup.env --profile scheduler --profile monitoring --profile olap --profile full_setup up -d
        ;;
    esac
}

check_services_health() {
    HYPERSWITCH_BASE_URL="http://localhost:8080"
    HYPERSWITCH_HEALTH_URL="${HYPERSWITCH_BASE_URL}/health"
    HYPERSWITCH_DEEP_HEALTH_URL="${HYPERSWITCH_BASE_URL}/health/ready"
    local is_success=true
    # Basic health check
    health_response=$(curl --silent --fail "${HYPERSWITCH_HEALTH_URL}") || is_success=false
    if [ "${health_response}" != "health is good" ]; then
        is_success=false
    fi

    # Deep health check
    deep_health_response=$(curl --silent --fail "${HYPERSWITCH_DEEP_HEALTH_URL}") || is_success=false
    if [[ "$(echo "${deep_health_response}" | jq --raw-output '.error')" != "null" ]]; then
        is_success=false
    fi

    # Extract version
    if [ "${is_success}" = true ]; then
        VERSION=$(curl --silent --output /dev/null --request GET --write-out '%header{x-hyperswitch-version}' "${HYPERSWITCH_BASE_URL}" | sed 's/-dirty$//')
        INSTALLATION_STATUS="success"
        scarf_call
    fi
    print_access_info
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
    fi

    printf "  • ${GREEN}${BOLD}App Server${NC}: ${BLUE}${BOLD}http://localhost:8080${NC}\n"

    if [ "$PROFILE" = "full" ]; then
        printf "  • ${GREEN}${BOLD}Monitoring (Grafana)${NC}: ${BLUE}${BOLD}http://localhost:3000${NC}\n"
    fi
    printf "\n"

    # Default user credentials
    printf "            Use the following credentials:\n"
    printf "            Email:    demo@hyperswitch.com\n"
    printf "            Password: Hyperswitch@123\n"

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
scarf_call
show_banner
detect_docker_compose
check_prerequisites
setup_config
source .oneclick-setup.env
select_profile
start_services
check_services_health
