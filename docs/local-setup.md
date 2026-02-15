# Local Development Setup

## Prerequisites

- Node.js (16+)
- Docker
- Homebrew (on macOS)

## Starting the Development Environment

If you have Homebrew Redis running on port 6379, stop it first to avoid conflicts:

```bash
brew services stop redis || true
```

Then start the services:

```bash
docker compose -f docker-compose-development.yml up -d
```

Verify the server is running:

```bash
curl -s http://localhost:8080/health
```

## Troubleshooting

- If port 6379 is busy, ensure no local Redis is running
- If Docker containers fail, try `docker system prune` to clean up