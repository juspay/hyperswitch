# One-Click Docker Setup Guide

This document provides detailed information about the one-click setup script for Hyperswitch.

## Overview

The `setup.sh` script simplifies the process of setting up Hyperswitch in a local development or testing environment. It provides an interactive setup experience that handles checking prerequisites, configuring the environment, and starting the necessary services.

## Features

- **Prerequisite Checking**: Verifies Docker/Podman and Docker/Podman Compose installation.
- **Port Availability Check**: Ensures required ports are available to avoid conflicts.
- **Configuration Management**: Automatically sets up necessary configuration files.
- **Multiple Deployment Profiles**: Choose the right setup for your needs.
- **Health Checking**: Verifies services are running and healthy.
- **Detailed Feedback**: Provides clear output and helpful error messages.

## Deployment Profiles

The script offers four deployment profiles to match your needs:

### 1. Standard (Recommended)
- **Services**: App server + Control Center + Web SDK (includes PostgreSQL, Redis)
- **Best for**: General development and testing
- **Resources required**: Medium

### 2. Full
- **Services**: Standard + Monitoring (Grafana, Prometheus) + Scheduler
- **Best for**: Complete system testing
- **Resources required**: Higher

### 3. Standalone App Server
- **Services**: Hyperswitch server, PostgreSQL, Redis
- **Best for**: Testing basic API functionality
- **Resources required**: Lower


## Troubleshooting

### Common Issues

1. **Docker not running**
   - **Error**: "Cannot connect to the Docker/Podman daemon"
   - **Solution**: Start the Docker daemon/Docker Desktop or Use Orbstack.

2. **Port conflicts**
   - **Error**: "The following ports are already in use: [port list]"
   - **Solution**: Stop services using those ports or choose different ports.

3. **Server not becoming healthy**
   - **Error**: "Hyperswitch server did not become healthy in the expected time."
   - **Solution**: Check logs with `docker compose logs hyperswitch-server` or  `podman compose logs hyperswitch-server`.

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

- Stop all services: `docker/podman compose down`
- Start specific services: `docker/podman compose up -d [service-name]`
- Restart a service: `docker/podman compose restart [service-name]`

## Next Steps

After running the setup script:

1. Verify the server is running: `curl --head --request GET 'http://localhost:8080/health'`.
2. Access the Control Center at `http://localhost:9000`.
3. Configure payment connectors in the Control Center.
4. Try a test payment using the demo store.
