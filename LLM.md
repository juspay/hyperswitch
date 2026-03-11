# LLM.md - Hanzo Payments

## Overview
This is just to run automated newman tests for this service

## Tech Stack
- **Language**: TypeScript/JavaScript

## Build & Run
```bash
npm install && npm run build
npm test
```

## Structure
```
payments/
  CHANGELOG.md
  Cargo.lock
  Cargo.toml
  Dockerfile
  INSTALL_dependencies.sh
  LICENSE
  LLM.md
  Makefile
  NOTICE
  README.md
  add_connector.md
  add_connector_updated.md
  api-reference/
  aws/
  cog.toml
```

## Key Files
- `README.md` -- Project documentation
- `package.json` -- Dependencies and scripts
- `Cargo.toml` -- Rust crate config
- `Makefile` -- Build automation
- `Dockerfile` -- Container build
