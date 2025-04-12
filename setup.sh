#!/bin/bash
set -e

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
    echo "  _   _                                      _ _       _     "
    echo " | | | |_   _ _ __   ___ _ __ _____      __(_) |_ ___| |__  "
    echo " | |_| | | | | '_ \ / _ \ '__/ __\ \ /\ / /| | __/ __| '_ \ "
    echo " |  _  | |_| | |_) |  __/ |  \__ \\ V  V / | | || (__| | | |"
    echo " |_| |_|\__, | .__/ \___|_|  |___/ \_/\_/  |_|\__\___|_| |_|"
    echo "        |___/|_|                                            "
    echo -e "${NC}"
    echo -e "🚀 ${BLUE}One-Click Docker Setup${NC} 🚀"
    echo
}

check_prerequisites() {
    echo_info "Checking prerequisites..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        echo_error "Docker is not installed. Please install Docker first."
        echo_info "Visit https://docs.docker.com/get-docker/ for installation instructions."
        exit 1
    fi
    echo_success "Docker is installed."
    
    # Check Docker Compose
    if ! docker compose version &> /dev/null; then
        echo_warning "Docker Compose V2 not detected. Checking for docker-compose..."
        if ! command -v docker-compose &> /dev/null; then
            echo_error "Docker Compose is not installed. Please install Docker Compose first."
            echo_info "Visit https://docs.docker.com/compose/install/ for installation instructions."
            exit 1
        fi
        echo_warning "Using legacy docker-compose. Consider upgrading to Docker Compose V2."
        DOCKER_COMPOSE="docker-compose"
    else
        echo_success "Docker Compose V2 is installed."
        DOCKER_COMPOSE="docker compose"
    fi
    
    # Check ports
    echo_info "Checking if required ports are available..."
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
        read -p "Do you want to continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    else
        echo_success "All required ports are available."
    fi
}

setup_config() {
    echo_info "Setting up configuration..."
    
    if [ ! -f "config/docker_compose.toml" ]; then
        if [ -f "config/config.example.toml" ]; then
            echo_info "Creating docker_compose.toml from example config..."
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
    echo_info "Select a deployment profile:"
    echo "1) Minimal (Core services only - Hyperswitch server, PostgreSQL, Redis)"
    echo "2) Standard (Minimal + Control Center + Web SDK)"
    echo "3) Full (Standard + Monitoring + Scheduler)"
    echo "4) Development (Build from source)"
    
    local profile_selected=false
    while [ "$profile_selected" = false ]; do
        read -p "Enter your choice [2]: " profile_choice
        profile_choice=${profile_choice:-2}
        
        case $profile_choice in
            1)
                PROFILE="minimal"
                profile_selected=true
                ;;
            2)
                PROFILE="standard"
                profile_selected=true
                ;;
            3)
                PROFILE="full"
                profile_selected=true
                ;;
            4)
                PROFILE="development"
                profile_selected=true
                ;;
            *)
                echo_error "Invalid choice. Please enter 1, 2, 3, or 4."
                ;;
        esac
    done
    
    echo_success "Selected profile: $PROFILE"
}

start_services() {
    echo_info "Starting Hyperswitch services with profile: $PROFILE"
    
    case $PROFILE in
        minimal)
            echo_info "Starting core services only..."
            $DOCKER_COMPOSE up -d pg redis-standalone migration_runner hyperswitch-server
            ;;
        standard)
            echo_info "Starting standard services..."
            $DOCKER_COMPOSE up -d pg redis-standalone migration_runner hyperswitch-server hyperswitch-web hyperswitch-control-center
            ;;
        full)
            echo_info "Starting all services..."
            $DOCKER_COMPOSE up -d pg redis-standalone migration_runner hyperswitch-server hyperswitch-web hyperswitch-control-center --profile scheduler --profile monitoring
            ;;
        development)
            echo_info "Starting development environment (build from source)..."
            $DOCKER_COMPOSE -f docker-compose-development.yml up -d
            ;;
    esac
}

check_services_health() {
    echo_info "Checking services health..."
    
    # Wait for the hyperswitch-server to be healthy
    MAX_RETRIES=30
    RETRY_INTERVAL=5
    RETRIES=0
    
    echo_info "Waiting for Hyperswitch server to become healthy..."
    while [ $RETRIES -lt $MAX_RETRIES ]; do
        if curl --silent --head --request GET 'http://localhost:8080/health' | grep "200 OK" > /dev/null; then
            echo_success "Hyperswitch server is healthy!"
            break
        fi
        
        RETRIES=$((RETRIES+1))
        if [ $RETRIES -eq $MAX_RETRIES ]; then
            echo_error "Hyperswitch server did not become healthy in the expected time."
            echo_info "Check logs with: docker compose logs hyperswitch-server"
            echo_warning "The setup process will continue, but some services might not work correctly."
        else
            echo_info "Waiting for server to become healthy... ($RETRIES/$MAX_RETRIES)"
            sleep $RETRY_INTERVAL
        fi
    done
}

print_access_info() {
    echo_info "Setup complete! You can access Hyperswitch services at:"
    echo "  • API Server: http://localhost:8080"
    
    if [ "$PROFILE" != "minimal" ]; then
        echo "  • Control Center: http://localhost:9000"
        echo "  • Web SDK Demo: http://localhost:9050"
    fi
    
    if [ "$PROFILE" = "full" ] || [ "$PROFILE" = "development" ]; then
        echo "  • Monitoring (Grafana): http://localhost:3000"
    fi
    
    echo_info "To verify that the server is running correctly, run:"
    echo "  curl --head --request GET 'http://localhost:8080/health'"
    
    echo_info "To view logs, run:"
    echo "  docker compose logs -f hyperswitch-server"
    
    echo_info "To stop all services, run:"
    echo "  docker compose down"
}

# Main execution flow
show_banner
check_prerequisites
setup_config
select_profile
start_services
check_services_health
print_access_info
echo_success "Hyperswitch is now ready to use!"