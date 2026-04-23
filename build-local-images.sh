#!/usr/bin/env bash
set -Eeuo pipefail

# =============================================================================
# Hyperswitch Local Docker Image Builder
# =============================================================================
#
# DESCRIPTION:
#   This script builds Hyperswitch Docker images locally with custom version tags.
#   It builds the router, scheduler (producer/consumer), and drainer binaries.
#
# USAGE:
#   ./build-local-images.sh [OPTIONS] [VERSION_TAG]
#
# ARGUMENTS:
#   VERSION_TAG     Custom tag for the images (e.g., v1.0.0-local, mycompany-1.2.3)
#                   If not provided, you'll be prompted interactively.
#
# OPTIONS:
#   -h, --help      Show this help message and exit
#   -r, --registry  Specify image registry (default: localhost)
#   -f, --features  Extra Cargo features to enable during build
#
# EXAMPLES:
#   # Build with interactive prompt for version tag
#   ./build-local-images.sh
#
#   # Build with specific version tag
#   ./build-local-images.sh v1.0.0-local
#
#   # Build with custom registry
#   ./build-local-images.sh --registry myregistry.io v1.0.0-custom
#
#   # Build with extra features
#   ./build-local-images.sh --features "aws_kms" v1.0.0-aws
#
# ENVIRONMENT VARIABLES:
#   IMAGE_REGISTRY      Docker registry prefix (default: localhost)
#   EXTRA_FEATURES      Additional Cargo features to enable
#   VERSION_FEATURE_SET Feature set version (default: v1)
#
# DEPLOYMENT:
#   After building, deploy using one of these methods:
#
#   1. Full setup (all services):
#      export CUSTOM_VERSION=<your-tag>
#      docker compose -f docker-compose.yml -f docker-compose.custom-images.yml up -d
#
#   2. Lightweight setup (server + observability only):
#      export CUSTOM_VERSION=<your-tag>
#      docker compose -f docker-compose-lightweight.yml -f docker-compose.custom-images.yml up -d
#
# OUTPUT IMAGES:
#   - ${IMAGE_REGISTRY}/hyperswitch-router:${VERSION_TAG}
#   - ${IMAGE_REGISTRY}/hyperswitch-producer:${VERSION_TAG}
#   - ${IMAGE_REGISTRY}/hyperswitch-consumer:${VERSION_TAG}
#   - ${IMAGE_REGISTRY}/hyperswitch-drainer:${VERSION_TAG}
#
# NOTES:
#   - Requires Docker to be installed and running
#   - Build time: 15-45 minutes depending on machine specs
#   - Uses multi-stage build with Rust toolchain
# =============================================================================

show_help() {
    head -n 56 "$0" | tail -n 52 | sed 's/^# //' | sed 's/^#//'
    exit 0
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

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

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            ;;
        -r|--registry)
            IMAGE_REGISTRY="$2"
            shift 2
            ;;
        -f|--features)
            EXTRA_FEATURES="$2"
            shift 2
            ;;
        -*)
            echo_error "Unknown option: $1"
            echo_info "Use -h or --help for usage information"
            exit 1
            ;;
        *)
            VERSION_TAG="$1"
            shift
            ;;
    esac
done

# Get version tag from argument or prompt
if [ -z "${VERSION_TAG:-}" ]; then
    echo_info "No version tag provided as argument."
    echo -n "Enter custom version tag (e.g., v1.0.0-local, mycompany-1.2.3): "
    read -r VERSION_TAG
fi

if [ -z "$VERSION_TAG" ]; then
    VERSION_TAG="local"
    echo_warning "No version tag provided, using default: $VERSION_TAG"
fi

# Validate version tag format
if [[ ! "$VERSION_TAG" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo_error "Invalid version tag format. Use only alphanumeric characters, dots, underscores, and hyphens."
    exit 1
fi

# Set image names with custom tag
IMAGE_REGISTRY="${IMAGE_REGISTRY:-localhost}"
ROUTER_IMAGE="${IMAGE_REGISTRY}/hyperswitch-router:${VERSION_TAG}"
PRODUCER_IMAGE="${IMAGE_REGISTRY}/hyperswitch-producer:${VERSION_TAG}"
CONSUMER_IMAGE="${IMAGE_REGISTRY}/hyperswitch-consumer:${VERSION_TAG}"
DRAINER_IMAGE="${IMAGE_REGISTRY}/hyperswitch-drainer:${VERSION_TAG}"

echo_info "Building Hyperswitch images with version tag: ${BOLD}${VERSION_TAG}${NC}"
echo_info "Image registry: ${IMAGE_REGISTRY}"
echo ""

# Check if Docker is installed
if ! command -v docker &>/dev/null; then
    echo_error "Docker is not installed. Please install Docker to proceed."
    exit 1
fi

echo_success "Docker is installed."
echo ""

# Function to build an image
build_image() {
    local binary_name=$1
    local image_name=$2
    local scheduler_flow=${3:-""}

    echo_info "Building ${BOLD}${binary_name}${NC} image..."
    echo_info "Target image: ${image_name}"

    local build_args=(
        --build-arg "BINARY=${binary_name}"
        --tag "${image_name}"
        --file Dockerfile
    )

    if [ -n "$scheduler_flow" ]; then
        build_args+=(--build-arg "SCHEDULER_FLOW=${scheduler_flow}")
    fi

    # Add any extra features if needed
    if [ -n "${EXTRA_FEATURES:-}" ]; then
        build_args+=(--build-arg "EXTRA_FEATURES=${EXTRA_FEATURES}")
    fi

    # Set version feature set (v1, v2, etc.)
    if [ -n "${VERSION_FEATURE_SET:-}" ]; then
        build_args+=(--build-arg "VERSION_FEATURE_SET=${VERSION_FEATURE_SET}")
    fi

    docker build "${build_args[@]}" .

    if [ $? -eq 0 ]; then
        echo_success "Successfully built ${image_name}"
    else
        echo_error "Failed to build ${image_name}"
        exit 1
    fi
    echo ""
}

# Record start time
START_TIME=$(date +%s)

# Build Router image (main application)
build_image "router" "${ROUTER_IMAGE}"

# Build Producer image (scheduler producer)
build_image "scheduler" "${PRODUCER_IMAGE}" "producer"

# Build Consumer image (scheduler consumer)
build_image "scheduler" "${CONSUMER_IMAGE}" "consumer"

# Build Drainer image
echo_info "Checking if drainer binary exists in codebase..."
if grep -q "\[\[bin\]\]" Cargo.toml 2>/dev/null && grep -q "name = \"drainer\"" Cargo.toml 2>/dev/null; then
    build_image "drainer" "${DRAINER_IMAGE}"
else
    echo_warning "Drainer binary not found in Cargo.toml, skipping drainer image build."
fi

# Calculate build time
END_TIME=$(date +%s)
BUILD_TIME=$((END_TIME - START_TIME))
MINUTES=$((BUILD_TIME / 60))
SECONDS=$((BUILD_TIME % 60))

echo ""
echo_success "========================================"
echo_success "Build completed successfully!"
echo_success "========================================"
echo ""
echo_info "Build duration: ${MINUTES}m ${SECONDS}s"
echo ""
echo_info "Built images:"
echo "  • Router:    ${ROUTER_IMAGE}"
echo "  • Producer:  ${PRODUCER_IMAGE}"
echo "  • Consumer:  ${CONSUMER_IMAGE}"
echo "  • Drainer:   ${DRAINER_IMAGE}"
echo ""
echo_info "To use these images in docker-compose, run:"
echo "  docker compose -f docker-compose.yml -f docker-compose.custom-images.yml up -d"
echo ""
echo_info "Or with monitoring profile (Grafana, Loki, Prometheus):"
echo "  docker compose -f docker-compose.yml -f docker-compose.custom-images.yml --profile monitoring up -d"
echo ""
echo_info "Or with full stack (includes scheduler):"
echo "  docker compose -f docker-compose.yml -f docker-compose.custom-images.yml --profile scheduler --profile monitoring up -d"
echo ""

# Save the version tag to a file for reference
echo "${VERSION_TAG}" > .custom-version
echo "${ROUTER_IMAGE}" > .router-image
echo "${PRODUCER_IMAGE}" > .producer-image
echo "${CONSUMER_IMAGE}" > .consumer-image
echo "${DRAINER_IMAGE}" > .drainer-image

# Generate docker-compose override file
cat > docker-compose.custom-images.yml << EOF
# Auto-generated docker-compose override for custom local images
# Version: ${VERSION_TAG}
# Generated: $(date)
#
# Use this file with: docker compose -f docker-compose.yml -f docker-compose.custom-images.yml up -d

services:
  hyperswitch-server:
    image: ${ROUTER_IMAGE}
    pull_policy: never

  hyperswitch-producer:
    image: ${PRODUCER_IMAGE}
    pull_policy: never

  hyperswitch-consumer:
    image: ${CONSUMER_IMAGE}
    pull_policy: never

  hyperswitch-drainer:
    image: ${DRAINER_IMAGE}
    pull_policy: never
EOF

echo_success "Created docker-compose.custom-images.yml"
