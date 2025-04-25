#! /usr/bin/env bash
set -euo pipefail

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Alias for docker to use podman
alias docker=podman

# Function to print colorful messages
echo_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

echo_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

echo_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_banner() {
    echo -e "${BLUE}${BOLD}"
    echo "" 
    echo "        #             "
    echo "        # #    #  ####  #####    ##   #   #  "
    echo "        # #    # #      #    #  #  #   # #   "
    echo "        # #    #  ####  #    # #    #   #    "
    echo "  #     # #    #      # #####  ######   #    "
    echo "  #     # #    # #    # #      #    #   #    "
    echo "   #####   ####   ####  #      #    #   #    "
    echo "" 
    echo "" 
    echo "  #     # #   # #####  ###### #####   ####  #    # # #####  ####  #    # "
    echo "  #     #  # #  #    # #      #    # #      #    # #   #   #    # #    # "
    echo "  #     #   #   #    # #####  #    #  ####  #    # #   #   #      ###### "
    echo "  #######   #   #####  #      #####       # # ## # #   #   #      #    # "
    echo "  #     #   #   #      #      #   #  #    # ##  ## #   #   #    # #    # "
    echo "  #     #   #   #      ###### #    #  ####  #    # #   #    ####  #    # "
    echo ""                                                                                                           
    sleep 1
    echo -e "${NC}"
    echo -e "🚀 ${BLUE}One-Click Docker Setup${NC} 🚀"
    echo
}

# Detect Docker Compose version
detect_docker_compose() {
    # Check Podman or Docker
    if command -v podman &> /dev/null; then
        CONTAINER_ENGINE="podman"
        echo_success "Podman is installed."
    elif command -v docker &> /dev/null; then
        CONTAINER_ENGINE="docker"
        echo_success "Docker is installed."
    else
        echo_error "Neither Podman nor Docker is installed. Please install one of them to proceed."
        echo_info "Visit https://podman.io/docs/installation or https://docs.docker.com/get-docker/ for installation instructions."
        echo_info "After installation, re-run this script: scripts/setup.sh"
        exit 1
    fi

    # Check Docker Compose or Podman Compose
    if $CONTAINER_ENGINE compose version &> /dev/null; then
        DOCKER_COMPOSE="$CONTAINER_ENGINE compose"
        echo_success "Compose is installed for $CONTAINER_ENGINE."
    else
        echo_error "Compose is not installed for $CONTAINER_ENGINE. Please install $CONTAINER_ENGINE compose to proceed."
        if [ "$CONTAINER_ENGINE" = "docker" ]; then
            echo_info "Visit https://docs.docker.com/compose/install/ for installation instructions."
        elif [ "$CONTAINER_ENGINE" = "podman" ]; then
            echo_info "Visit https://podman-desktop.io/docs/compose/setting-up-compose for installation instructions."
    fi
    echo_info "After installation, re-run this script: scripts/setup.sh"
    exit 1
    fi
}

check_prerequisites() {
    # Check curl
    if ! command -v curl &> /dev/null; then
        echo_error "curl is not installed. Please install curl to proceed."
        exit 1
    fi
    echo_success "curl is installed."
    
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
            break
        fi
    done
    
    if [ ${#unavailable_ports[@]} -ne 0 ]; then
        echo_warning "The following ports are already in use: ${unavailable_ports[*]}"
        echo_warning "This might cause conflicts with Hyperswitch services."
        read -p "Do you want to continue anyway? (y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo_success "All required ports are available."
    fi
}

setup_config() {
    if [ ! -f "config/docker_compose.toml" ]; then
        echo_error "Configuration file 'config/docker_compose.toml' not found. Please ensure the file exists and is correctly configured."
        exit 1
    fi
}

select_profile() {    
    echo ""    
    echo "Select a setup option:"
    echo -e "1) Standard Setup: ${BLUE}[Recommended]${NC} Ideal for quick trial. Services included: ${BLUE}App Server, Control Center, Unified Checkout, PostgreSQL and Redis${NC}"
    echo -e "2) Full Stack Setup: Ideal for comprehensive end-to-end payment testing. Services included: ${BLUE}Everything in Standard, Monitoring and Scheduler${NC}"
    echo -e "3) Standalone App Server: Ideal for API-first integration testing. Services included: ${BLUE}App server, PostgreSQL and Redis)${NC}"
    echo ""
    local profile_selected=false
    while [ "$profile_selected" = false ]; do
        read -p "Enter your choice (1-3): " profile_choice
        profile_choice=${profile_choice:-1}
        
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
            $DOCKER_COMPOSE --profile scheduler --profile monitoring --profile olap up -d
            ;;
    esac
}

check_services_health() {
    # Wait for the hyperswitch-server to be healthy
    MAX_RETRIES=30
    RETRY_INTERVAL=5
    RETRIES=0
    
    while [ $RETRIES -lt $MAX_RETRIES ]; do
        if curl --silent --head --request GET 'http://localhost:8080/health' | grep "200 OK" > /dev/null; then
            echo_success "Hyperswitch server is healthy!"
            print_access_info  # Call print_access_info only when the server is healthy
            return
        fi
        
        RETRIES=$((RETRIES+1))
        if [ $RETRIES -eq $MAX_RETRIES ]; then
            echo ""
            echo_error "${RED}${BOLD}Hyperswitch server did not become healthy in the expected time."
            echo -e "Check logs with: $DOCKER_COMPOSE logs hyperswitch-server, Or reach out to us on slack(https://hyperswitch-io.slack.com/) for help."
            echo -e "The setup process will continue, but some services might not work correctly.${NC}"
            echo ""
        else
            echo "Waiting for server to become healthy... ($RETRIES/$MAX_RETRIES)"
            sleep $RETRY_INTERVAL
        fi
    done
}

print_access_info() {
    echo ""
    echo -e "${GREEN}${BOLD}Setup complete! You can access Hyperswitch services at:${NC}"
    echo ""
    
    if [ "$PROFILE" != "standalone" ]; then
        echo -e "  • ${GREEN}${BOLD}Control Center${NC}: ${BLUE}${BOLD}http://localhost:9000${NC}"
    fi
    
    echo -e "  • ${GREEN}${BOLD}App Server${NC}: ${BLUE}${BOLD}http://localhost:8080${NC}"
    
    if [ "$PROFILE" != "standalone" ]; then
        echo -e "  • ${GREEN}${BOLD}Unified Checkout${NC}: ${BLUE}${BOLD}http://localhost:9060${NC}"
    fi
    
    if [ "$PROFILE" = "full" ]; then
        echo -e "  • ${GREEN}${BOLD}Monitoring (Grafana)${NC}: ${BLUE}${BOLD}http://localhost:3000${NC}"
    fi
    echo ""
    echo_info "To stop all services, run: $DOCKER_COMPOSE down"
}

# Main execution flow
show_banner
detect_docker_compose
check_prerequisites
setup_config
select_profile
start_services
check_services_health  # This will call print_access_info if the server is healthy
