# = Parameters
# Override envars using -e

#
# = Common
#

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)


ROOT_DIR_WITH_SLASH := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))
ROOT_DIR := $(realpath $(ROOT_DIR_WITH_SLASH))

#
# = Targets
#

.PHONY : \
	doc \
	fmt \
	clippy \
	test \
	audit \
	git.sync \
	build \
	push \
	shell \
	run \
	start \
	stop \
	rm \
	release


# Check a local package and all of its dependencies for errors
# 
# Usage :
#	make check
check:
	cargo check


# Compile application for running on local machine
#
# Usage :
#	make build
build :
	cargo build

# Generate crates documentation from Rust sources.
#
# Usage :
#	make doc [private=(yes|no)] [open=(yes|no)] [clean=(no|yes)]

doc :
ifeq ($(clean),yes)
	@rm -rf target/doc/
endif
	cargo doc --all-features --package router \
		$(if $(call eq,$(private),no),,--document-private-items) \
		$(if $(call eq,$(open),no),,--open)

# Format Rust sources with rustfmt.
#
# Usage :
#	make fmt [dry_run=(no|yes)]

fmt :
	cargo +nightly fmt --all $(if $(call eq,$(dry_run),yes),-- --check,)

# Lint Rust sources with Clippy.
#
# Usage :
#	make clippy

clippy :
	cargo clippy --all-features --all-targets -- -D warnings

# Build the DSL crate as a WebAssembly JS library
#
# Usage :
# 	make euclid-wasm

euclid-wasm:
	wasm-pack build --target web --out-dir $(ROOT_DIR)/wasm --out-name euclid $(ROOT_DIR)/crates/euclid_wasm  -- --features dummy_connector

# Run Rust tests of project.
#
# Usage :
#	make test

test :
	cargo test --all-features


# Next-generation test runner for Rust.
# cargo nextest ignores the doctests at the moment. So if you are using it locally you also have to run `cargo test --doc`.
# Usage:
# 	make nextest

nextest:
	cargo nextest run

# Run format clippy test and tests.
#
# Usage :
#	make precommit

precommit : fmt clippy test


hack:
	cargo hack check --workspace --each-feature --all-targets

# Run database migrations using `diesel_cli`.
# Assumes `diesel_cli` is already installed.
#
# Usage :
#	make migrate [database-url=<PSQL connection string>] [locked-schema=<yes|no>]

# This proceeds as follows:
# 	Creates a temporary migrations directory, cleans it up if it already exists
# 	Copies all migrations to the temporary migrations directory and runs migrations
# 	Cleans up migrations, removing tmp directory if empty, ignoring otherwise
migrate:
	mkdir -p $(ROOT_DIR)/tmp/migrations
	find $(ROOT_DIR)/tmp/migrations/ -mindepth 1 -delete

	cp -r $(ROOT_DIR)/migrations/. $(ROOT_DIR)/v2_migrations/. $(ROOT_DIR)/tmp/migrations/
	diesel migration run --migration-dir=$(ROOT_DIR)/tmp/migrations \
		$(if $(strip $(database-url)),--database-url="$(database-url)",) \
		$(if $(strip $(call eq,$(locked-schema),yes)),--locked-schema,)

	rm -r $(ROOT_DIR)/tmp/migrations
	rmdir $(ROOT_DIR)/tmp 2>/dev/null || true

redo_migrate: 
	mkdir -p $(ROOT_DIR)/tmp/migrations
	find $(ROOT_DIR)/tmp/migrations/ -mindepth 1 -delete

	cp -r $(ROOT_DIR)/migrations/. $(ROOT_DIR)/v2_migrations/. $(ROOT_DIR)/tmp/migrations/
	diesel migration redo --all --migration-dir=$(ROOT_DIR)/tmp/migrations \
		$(if $(strip $(database-url)),--database-url="$(database-url)",) \
		$(if $(strip $(call eq,$(locked-schema),yes)),--locked-schema,)

	rm -r $(ROOT_DIR)/tmp/migrations
	rmdir $(ROOT_DIR)/tmp 2>/dev/null || true	
