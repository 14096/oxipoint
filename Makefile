.PHONY: all build release test lint fmt fmt-check doc doc-open check features run run-device clean help

all: fmt-check lint test

help: ## Show available targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-12s %s\n", $$1, $$2}'

build: ## Build (debug)
	cargo build

release: ## Build optimized binary
	cargo build --release

test: ## Run unit tests and doctests
	cargo test

lint: ## Run clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

fmt: ## Format the code
	cargo fmt

fmt-check: ## Check formatting
	cargo fmt --check

doc: ## Build rustdoc (warnings denied)
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

doc-open: ## Build rustdoc and open it in the browser
	cargo doc --no-deps --open

check: ## Type-check without default features
	cargo check --lib --no-default-features

features: ## Check main feature combinations
	cargo check --lib --no-default-features --features graph,auth-password
	cargo check --lib --no-default-features --features sharepoint,auth-device-code,auth-browser,auth-certificate

run: ## Run the demo CLI (browser login); reads .env / RSP_* env vars or ARGS="<site-url> --client-id ID"
	cargo run -- $(ARGS)

run-device: ## Run the demo CLI with device code login
	cargo run -- --device-code $(ARGS)

clean: ## Remove build artifacts
	cargo clean
