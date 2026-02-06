.PHONY: help build test lint clean install release docker run-dev

help:
	@echo "Orchestr8 - Universal Runtime Control Plane"
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

build:
	@echo "Building debug binary..."
	cargo build

release:
	@echo "Building release binary..."
	cargo build --release
	@ls -lh target/release/orchestr8

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
	rm -rf .orchestr8/
	rm -f state.json

install: release
	@echo "Installing orchestr8 to /usr/local/bin..."
	sudo cp target/release/orchestr8 /usr/local/bin/orchestr8
	@echo "Installed successfully!"
	@orchestr8 --version

docker:
	@echo "Building Docker image..."
	docker build -f Dockerfile.orchestr8 -t orchestr8:latest .
	@echo "Built: orchestr8:latest"

run-dev:
	@echo "Running orchestr8 in development mode..."
	cargo run -- -v --help

ci: test lint
	@echo "CI checks passed!"

# Development workflow
watch:
	@echo "Watching for changes..."
	cargo watch -x 'build' -x 'test' -x 'clippy'

# Quick validation
validate:
	@echo "Validating example workloads..."
	cargo run --release -- validate --spec workload.yaml
	cargo run --release -- validate --spec workload-k8s.yaml
	cargo run --release -- validate --spec workload-kubevirt.yaml
	cargo run --release -- validate --spec workload-metal.yaml

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
