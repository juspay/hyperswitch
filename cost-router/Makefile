# Convenience targets. Run from the repo root as `make -C cost-router demo`, or from cost-router/.
.PHONY: demo up down test build

build:
	npm install --no-audit --no-fund
	npm run build

test: build
	npm test

up:
	bash scripts/up.sh

down:
	bash scripts/down.sh

# Stack + install + build + end-to-end demo. The first run is dominated by image downloads.
demo: up build
	bash scripts/demo.sh
