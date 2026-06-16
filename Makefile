.PHONY: help build test lint clean install release docker run-dev \
	confidential-validate confidential-fabric-e2e confidential-cluster-e2e reference-cluster-e2e reference-cluster-live reference-cluster-live-verify \
	post-deploy-verify remote-post-deploy-verify remote-reference-verify api-live-test api-live-test-remote \
	test-remote-smoke test-remote-quick test-remote-all \
	deploy-remote deploy-all-remote test-platform-remote

DEPLOY_HOST ?= 212.8.252.194
DEPLOY_USER ?= sus

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
	@echo "  ci         - Run CI checks (test + lint + dashboard glass guard)"
	@echo "  confidential-validate      - Validate examples/confidential-*.yaml"
	@echo "  confidential-fabric-e2e    - API smoke (AETHER_API, server running)"
	@echo "  confidential-cluster-e2e   - CLI placement/migrate checks (no live deploy)"
	@echo "  reference-cluster-e2e    - Metal3/KubeVirt + confidential kata validate (dry-run by default)"
	@echo "  reference-cluster-live     - Live Metal3/KubeVirt smoke (requires kubeconfig)"
	@echo "  reference-cluster-live-verify - Live smoke + post-deploy API verify"
	@echo "  post-deploy-verify         - API smoke against AETHER_API (default localhost:5090)"
	@echo "  api-live-test              - Live E2E orchestrator (default: all live tiers + Playwright)"
	@echo "  api-live-test-remote       - api-live-test against DEPLOY_HOST (212.8.252.194)"
	@echo "  remote-post-deploy-verify    - Post-deploy verify against 212.8.252.194:30090"
	@echo "  remote-reference-verify      - Remote k8s live lab + post-deploy verify"
	@echo "  test-remote-smoke            - AETHER_TEST_TIERS=smoke on staging host"
	@echo "  test-remote-quick            - AETHER_TEST_TIERS=quick"
	@echo "  test-remote-all              - AETHER_TEST_TIERS=full (nightly, includes Playwright)"
	@echo "  See docs/TEST_PLAN.md for full tier matrix"
	@echo "  deploy-remote                - Deploy to DEPLOY_HOST (default 212.8.252.194)"
	@echo "  deploy-all-remote            - Full orchestrated remote deploy + health checks"
	@echo "  test-platform-remote         - Aether + Hermes full remote E2E on DEPLOY_HOST"
	@echo "  deploy-reference-ingress     - Deploy remote with Ingress/TLS (AETHER_DEPLOY_ENV)"
	@echo "  deploy-reference-sso         - Deploy remote with mock IdP SSO (AETHER_DEPLOY_ENV)"
	@echo "  deploy-with-ldap             - Deploy remote with LDAP/AD (AETHER_DEPLOY_ENV required)"
	@echo "  deploy-hosted-stripe-prod    - Deploy remote with Stripe production billing env"
	@echo "  deploy-confidential-snp-lab  - Validate confidential SNP lab (offline smoke)"
	@echo "  migration-dry-run-e2e        - Validate demo specs + dry-run migrate targets"

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

ci: test lint confidential-validate audit-warn dashboard-glass
	@echo "CI checks passed!"

dashboard-glass:
	@echo "Checking dashboard liquid glass surfaces..."
	cd web/dashboard && npm run check:hex-surfaces && npm run build

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

reference-cluster-live: release
	@chmod +x scripts/labs-live-smoke.sh scripts/reference-cluster-e2e.sh scripts/labs-e2e.sh 2>/dev/null || true
	@AETHER_LABS_LIVE=1 ./scripts/labs-live-smoke.sh

reference-cluster-live-verify: release
	@chmod +x scripts/reference-cluster-live-verify.sh scripts/labs-live-smoke.sh scripts/reference-cluster-e2e.sh scripts/labs-e2e.sh scripts/post-deploy-verify.sh 2>/dev/null || true
	@AETHER_LABS_LIVE=1 ./scripts/reference-cluster-live-verify.sh

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
	@chmod +x scripts/post-deploy-verify.sh scripts/lib/post-deploy-auth.sh 2>/dev/null || true
	@./scripts/post-deploy-verify.sh

api-live-test:
	@chmod +x scripts/api-live-test.sh scripts/lib/e2e-tier-runner.sh scripts/lib/post-deploy-auth.sh 2>/dev/null || true
	@./scripts/api-live-test.sh

api-live-test-remote:
	@chmod +x scripts/api-live-test.sh scripts/lib/e2e-tier-runner.sh scripts/lib/post-deploy-auth.sh 2>/dev/null || true
	@AETHER_REMOTE_HOST=$(DEPLOY_HOST) ./scripts/api-live-test.sh $(DEPLOY_HOST) $(DEPLOY_USER)

remote-post-deploy-verify:
	@chmod +x scripts/post-deploy-verify.sh scripts/lib/post-deploy-auth.sh 2>/dev/null || true
	@AETHER_API=http://212.8.252.194:30090 ./scripts/post-deploy-verify.sh

remote-reference-verify:
	@chmod +x scripts/remote-reference-verify.sh scripts/k8s-labs-e2e.sh scripts/post-deploy-verify.sh scripts/lib/post-deploy-auth.sh 2>/dev/null || true
	@./scripts/remote-reference-verify.sh

test-remote-smoke:
	@chmod +x scripts/test-all-features-remote.sh 2>/dev/null || true
	@AETHER_TEST_TIERS=smoke ./scripts/test-all-features-remote.sh $(DEPLOY_HOST) $(DEPLOY_USER)

test-remote-quick:
	@chmod +x scripts/test-all-features-remote.sh 2>/dev/null || true
	@AETHER_TEST_TIERS=quick ./scripts/test-all-features-remote.sh $(DEPLOY_HOST) $(DEPLOY_USER)

test-remote-all:
	@chmod +x scripts/test-all-features-remote.sh 2>/dev/null || true
	@AETHER_TEST_TIERS=full ./scripts/test-all-features-remote.sh $(DEPLOY_HOST) $(DEPLOY_USER)

deploy-remote:
	@chmod +x scripts/deploy-remote.sh scripts/lib/resolve-zyvor-sibling.sh 2>/dev/null || true
	@./scripts/deploy-remote.sh $(DEPLOY_HOST) $(DEPLOY_USER)

deploy-all-remote:
	@chmod +x scripts/deploy-all-remote.sh scripts/deploy-all.sh scripts/deploy-remote.sh \
		scripts/lib/resolve-zyvor-sibling.sh 2>/dev/null || true
	@./scripts/deploy-all-remote.sh $(DEPLOY_HOST) $(DEPLOY_USER)

test-platform-remote:
	@chmod +x scripts/test-platform-remote.sh scripts/test-all-features-remote.sh \
		scripts/lib/resolve-zyvor-sibling.sh 2>/dev/null || true
	@AETHER_TEST_TIERS=full HERMES_TEST_TIERS=full ./scripts/test-platform-remote.sh $(DEPLOY_HOST) $(DEPLOY_USER)

deploy-reference-ingress:
	@chmod +x scripts/deploy-reference-ingress.sh 2>/dev/null || true
	@echo "Usage: AETHER_DEPLOY_ENV=~/aether-ingress.env make deploy-reference-ingress HOST=<ip> USER=<ssh-user>"

deploy-reference-sso:
	@chmod +x scripts/deploy-reference-sso.sh 2>/dev/null || true
	@echo "Usage: AETHER_DEPLOY_ENV=~/aether-sso.env make deploy-reference-sso HOST=<ip> USER=<ssh-user>"

deploy-with-ldap:
	@chmod +x scripts/deploy-with-ldap.sh 2>/dev/null || true
	@echo "Usage: AETHER_DEPLOY_ENV=~/aether-ldap.env make deploy-with-ldap HOST=<ip> USER=<ssh-user>"

deploy-hosted-stripe-prod:
	@chmod +x scripts/deploy-hosted-stripe-prod.sh 2>/dev/null || true
	@echo "Usage: AETHER_DEPLOY_ENV=~/aether-stripe.env make deploy-hosted-stripe-prod HOST=<ip> USER=<ssh-user>"

deploy-confidential-snp-lab:
	@chmod +x scripts/deploy-confidential-snp-lab.sh scripts/confidential-cluster-e2e.sh scripts/lib/aether-confidential-smoke.sh 2>/dev/null || true
	@./scripts/deploy-confidential-snp-lab.sh

migration-dry-run-e2e:
	@cargo build --quiet 2>/dev/null || cargo build
	@chmod +x scripts/migration-dry-run-e2e.sh 2>/dev/null || true
	@AETHER_BIN=./target/debug/aether ./scripts/migration-dry-run-e2e.sh

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
