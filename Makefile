.PHONY: help build test lint clean install release docker run-dev \
	confidential-validate confidential-fabric-e2e confidential-cluster-e2e reference-cluster-e2e

help:
	@echo "Aether - Universal Runtime Control Plane"
	@echo ""
	@echo "Available targets:"
	@echo "  build      - Build debug binary"
	@echo "  release    - Build optimized release binary"
	@echo "  test       - Run all tests"
	@echo "  lint       - Run linters (fmt, clippy)"
	@echo "  clean      - Clean build artifacts"
	@echo "  install    - Install release binary to /usr/local/bin"
	@echo "  docker     - Build Docker image"
	@echo "  run-dev    - Run in development mode with verbose logging"
	@echo "  ci         - Run CI checks (test + lint)"
	@echo "  confidential-validate      - Validate examples/confidential-*.yaml"
	@echo "  confidential-fabric-e2e    - API smoke (AETHER_API, server running)"
	@echo "  confidential-cluster-e2e   - CLI placement/migrate checks (no live deploy)"
	@echo "  reference-cluster-e2e    - Metal3/KubeVirt + confidential kata validate (AETHER_LABS_LIVE=1 for live)"

build:
	@echo "Building debug binary..."
	cargo build

release:
	@echo "Building release binary..."
	cargo build --release
	@ls -lh target/release/aether

test:
	@echo "Running tests..."
	cargo test --verbose

lint:
	@echo "Running formatters and linters..."
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf .aether/
	rm -f state.json

install: release
	@echo "Installing aether to /usr/local/bin..."
	sudo cp target/release/aether /usr/local/bin/aether
	@echo "Installed successfully!"
	@aether --version

docker:
	@echo "Building Docker image..."
	docker build -f Dockerfile.aether -t aether:latest .
	@echo "Built: aether:latest"

run-dev:
	@echo "Running aether in development mode..."
	cargo run -- -v --help

ci: test lint confidential-validate audit-warn
	@echo "CI checks passed!"

audit-warn:
	@echo "Running cargo audit (non-blocking)..."
	-cargo audit

confidential-validate:
	@chmod +x scripts/validate-confidential-examples.sh 2>/dev/null || true
	@./scripts/validate-confidential-examples.sh

confidential-fabric-e2e:
	@chmod +x scripts/confidential-fabric-e2e.sh scripts/lib/aether-confidential-smoke.sh 2>/dev/null || true
	@./scripts/confidential-fabric-e2e.sh

confidential-cluster-e2e: release
	@chmod +x scripts/confidential-cluster-e2e.sh 2>/dev/null || true
	@./scripts/confidential-cluster-e2e.sh

deploy-remove-e2e:
	@chmod +x scripts/deploy-remove-e2e.sh 2>/dev/null || true
	@./scripts/deploy-remove-e2e.sh

reference-cluster-e2e: release
	@chmod +x scripts/reference-cluster-e2e.sh scripts/labs-e2e.sh scripts/confidential-cluster-e2e.sh 2>/dev/null || true
	@./scripts/reference-cluster-e2e.sh

# Development workflow
watch:
	@echo "Watching for changes..."
	cargo watch -x 'build' -x 'test' -x 'clippy'

# Quick validation
validate:
	@echo "Validating example workloads..."
	@chmod +x scripts/validate-schema-examples.sh 2>/dev/null || true
	@AETHER_BIN=./target/release/aether scripts/validate-schema-examples.sh

post-deploy-verify:
	@chmod +x scripts/post-deploy-verify.sh 2>/dev/null || true
	@./scripts/post-deploy-verify.sh

# Benchmark (if criterion is added)
bench:
	cargo bench

# Coverage report (requires tarpaulin)
coverage:
	@echo "Generating coverage report..."
	cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Html
	@echo "Coverage report: tarpaulin-report.html"

# Security audit
audit:
	@echo "Running security audit..."
	cargo audit

# Update dependencies
update:
	@echo "Updating dependencies..."
	cargo update

# Check for outdated dependencies
outdated:
	@echo "Checking for outdated dependencies..."
	cargo outdated

# Documentation
docs:
	@echo "Building documentation..."
	cargo doc --no-deps --open

# All checks before commit
pre-commit: lint test
	@echo "Pre-commit checks passed!"
