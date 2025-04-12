# One-Click Docker Setup Guide

This document provides detailed information about the one-click setup script for Hyperswitch.

## Overview

The `setup.sh` script is designed to simplify the process of setting up Hyperswitch in a local development or testing environment. It provides a guided, interactive setup experience that handles checking prerequisites, configuring the environment, and starting the necessary services.

## Features

- **Prerequisite Checking**: Automatically verifies Docker and Docker Compose installation
- **Port Availability Check**: Ensures required ports are available
- **Configuration Management**: Sets up necessary configuration files
- **Multiple Deployment Profiles**: Choose the right setup for your needs
- **Health Checking**: Verifies services are properly running
- **Detailed Feedback**: Clear output and helpful error messages

## Deployment Profiles

The script offers four different deployment profiles to match your needs:

### 1. Minimal
- **Services**: Hyperswitch server, PostgreSQL, Redis
- **Best for**: Testing basic API functionality
- **Resources required**: Lower

### 2. Standard (Default)
- **Services**: Minimal + Control Center + Web SDK
- **Best for**: General development and testing
- **Resources required**: Medium

### 3. Full
- **Services**: Standard + Monitoring (Grafana, Prometheus) + Scheduler
- **Best for**: Complete system testing
- **Resources required**: Higher

### 4. Development
- **Services**: Complete environment built from source
- **Best for**: Active development on Hyperswitch
- **Resources required**: Highest

## Troubleshooting

### Common Issues

1. **Docker not running**
   - Error: "Cannot connect to the Docker daemon"
   - Solution: Start the Docker daemon/Docker Desktop

2. **Port conflicts**
   - Error: "The following ports are already in use: [port list]"
   - Solution: Stop services using those ports or choose different ports

3. **Server not becoming healthy**
   - Error: "Hyperswitch server did not become healthy in the expected time"
   - Solution: Check logs with `docker compose logs hyperswitch-server`

### Viewing Logs

To view logs for any service:
```
docker compose logs -f [service-name]
```

Common service names:
- `hyperswitch-server`
- `pg` (PostgreSQL)
- `redis-standalone`
- `hyperswitch-control-center`

## Advanced Usage

### Environment Variables

You can set these environment variables before running the script:

- `DRAINER_INSTANCE_COUNT`: Number of drainer instances (default: 1)
- `REDIS_CLUSTER_COUNT`: Number of Redis cluster nodes (default: 3)

Example:
```
export DRAINER_INSTANCE_COUNT=2
./setup.sh
```

### Manual Service Control

After setup, you can manually control services:

- Stop all services: `docker compose down`
- Start specific services: `docker compose up -d [service-name]`
- Restart a service: `docker compose restart [service-name]`

## Next Steps

After running the setup script:

1. Verify the server is running: `curl --head --request GET 'http://localhost:8080/health'`
2. Access the Control Center at `http://localhost:9000`
3. Configure payment connectors in the Control Center
4. Try a test payment using the demo store