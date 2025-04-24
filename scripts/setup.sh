#!/bin/bash
set -euo pipefail

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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
    echo -e "${BLUE}"
    echo "" 
    echo " █████   █████                                                                  ███   █████             █████     "
    echo " ░░███   ░░███                                                                  ░░░   ░░███             ░░███      "
    echo "  ░███    ░███  █████ ████ ████████   ██████  ████████   █████  █████ ███ █████ ████  ███████    ██████  ░███████  "
    echo "  ░███████████ ░░███ ░███ ░░███░░███ ███░░███░░███░░███ ███░░  ░░███ ░███░░███ ░░███ ░░░███░    ███░░███ ░███░░███ "
    echo "  ░███░░░░░███  ░███ ░███  ░███ ░███░███████  ░███ ░░░ ░░█████  ░███ ░███ ░███  ░███   ░███    ░███ ░░░  ░███ ░███ "
    echo "  ░███    ░███  ░███ ░███  ░███ ░███░███░░░   ░███      ░░░░███ ░░███████████   ░███   ░███ ███░███  ███ ░███ ░███ "
    echo "  █████   █████ ░░███████  ░███████ ░░██████  █████     ██████   ░░████░████    █████  ░░█████ ░░██████  ████ █████ "
    echo " ░░░░░   ░░░░░   ░░░░░███  ░███░░░   ░░░░░░  ░░░░░     ░░░░░░     ░░░░ ░░░░    ░░░░░    ░░░░░   ░░░░░░  ░░░░ ░░░░░  "
    echo "                 ███ ░███  ░███                                                                                    "
    echo "                ░░██████   █████                                                                                    "
    echo "                  ░░░░░░   ░░░░░                                                                                    "                                                                                                      
    echo ""
    sleep 1
    echo -e "${NC}"
    echo -e "🚀 ${BLUE}One-Click Docker Setup${NC} 🚀"
    echo
}

# Detect Docker Compose version
detect_docker_compose() {
    # Check Docker
    if ! command -v docker &> /dev/null; then
        echo_error "Docker is not installed. Please install Docker first."
        echo_info "Visit https://docs.docker.com/get-docker/ for installation instructions."
        exit 1
    fi
    echo_success "Docker is installed."

    # Check Docker Compose
    if docker compose version &> /dev/null; then
        DOCKER_COMPOSE="docker compose"
        echo_success "Docker Compose is installed."
    else
        echo_error "Docker Compose is not installed. Please install Docker Compose(Alternatively use Orbstack) first."
        echo_info "Visit https://docs.docker.com/compose/install/ for installation instructions."
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
    required_ports=(8080 9000 9050 5432 6379)
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
        if [ -f "config/config.example.toml" ]; then
            echo "Creating docker_compose.toml from example config..."
            cp config/config.example.toml config/docker_compose.toml
            echo_success "Configuration file created."
        else
            echo_error "Example configuration file not found."
            exit 1
        fi
    else
        echo_success "Configuration file already exists."
    fi
}

select_profile() {
    echo "Select a deployment profile:"
    echo "1) Standard (Recommended - App server + Control Center + Web SDK)"
    echo "2) Full (Standard + Monitoring + Scheduler)"
    echo "3) Development (Build from source - may take up to 30 minutes)"
    echo "4) Standalone App Server (Core services only - Hyperswitch server, PostgreSQL, Redis)"
    
    local profile_selected=false
    while [ "$profile_selected" = false ]; do
        read -p "Enter your choice : " profile_choice
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
                PROFILE="development"
                profile_selected=true
                ;;
            4)
                PROFILE="standalone"
                profile_selected=true
                ;;
            *)
                echo_error "Invalid choice. Please enter 1, 2, 3, or 4."
                ;;
        esac
    done
    
    echo "Selected profile: $PROFILE"
}

start_services() {
    echo "Starting Hyperswitch services with profile: $PROFILE"
    
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
        development)
            $DOCKER_COMPOSE -f docker-compose-development.yml up -d
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
            break
        fi
        
        RETRIES=$((RETRIES+1))
        if [ $RETRIES -eq $MAX_RETRIES ]; then
            echo_error "Hyperswitch server did not become healthy in the expected time."
            echo_info "Check logs with: $DOCKER_COMPOSE logs hyperswitch-server"
            echo_warning "The setup process will continue, but some services might not work correctly."
        else
            echo "Waiting for server to become healthy... ($RETRIES/$MAX_RETRIES)"
            sleep $RETRY_INTERVAL
        fi
    done
}

print_access_info() {
    
    echo "Setup complete! You can access Hyperswitch services at:"
    
    if [ "$PROFILE" != "minimal" ]; then
        echo -e "  • ${GREEN}Control Center${NC}: ${BLUE}http://localhost:9000${NC}"
    fi
    
    echo -e "  • ${GREEN}API Server${NC}: ${BLUE}http://localhost:8080${NC}"
    
    if [ "$PROFILE" != "minimal" ]; then
        echo -e "  • ${GREEN}Web SDK Demo${NC}: ${BLUE}http://localhost:9050${NC}"
    fi
    
    if [ "$PROFILE" = "full" ] || [ "$PROFILE" = "development" ]; then
        echo -e "  • ${GREEN}Monitoring (Grafana)${NC}: ${BLUE}http://localhost:3000${NC}"
    fi
    echo "Hyperswitch is now ready to use!"
    echo_info "To stop all services, run:"
    echo "  $DOCKER_COMPOSE down"
}

# Main execution flow
show_banner
detect_docker_compose
check_prerequisites
setup_config
select_profile
start_services
check_services_health
print_access_info
