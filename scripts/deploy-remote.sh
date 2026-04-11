#!/bin/bash
# ============================================================================
# deploy-remote.sh — Full orchestr8 deployment to a remote server
# ============================================================================
# One command to fully set up a remote server:
#   1. Rsync repo to remote
#   2. Install Rust toolchain (rustup)
#   3. Install system dependencies (podman, kubectl)
#   4. Build release binary from source
#   5. Install to /usr/local/bin
#   6. Set up systemd service (API server on port 5090)
#   7. Open firewall port
#   8. Verify everything works
#
# Usage:
#   ./scripts/deploy-remote.sh <host> [user] [password]
#   ./scripts/deploy-remote.sh 185.165.240.5 sus mypassword
#   ./scripts/deploy-remote.sh 10.0.0.1 root                  # SSH key auth
#   ./scripts/deploy-remote.sh 10.0.0.1 root pass --quick     # skip deps, just rebuild + install
#   ./scripts/deploy-remote.sh 10.0.0.1 root pass --uninstall # remove orchestr8
#
# Environment variables:
#   DEPLOY_HOST=185.165.240.5
#   DEPLOY_USER=sus
#   DEPLOY_PASS=mypassword
#   DEPLOY_DIR=/home/sus/orchestr8  (auto-detected from login user)
# ============================================================================

set -euo pipefail

info()  { echo "  ✅ $*"; }
warn()  { echo "  ⚠️  $*"; }
error() { echo "  ❌ $*"; exit 1; }
step()  { echo ""; echo "  🔧 $*"; }

# ── Parse args ──
QUICK_MODE=false
UNINSTALL_MODE=false
POSITIONAL=()
for arg in "$@"; do
    case "$arg" in
        --quick)     QUICK_MODE=true ;;
        --uninstall) UNINSTALL_MODE=true ;;
        --help|-h)
            echo "Usage: $0 <host> [user] [password] [--quick|--uninstall]"
            echo ""
            echo "  --quick      Skip deps install (only rsync + cargo build + install)"
            echo "  --uninstall  Remove orchestr8 from remote server"
            echo ""
            echo "Full mode installs: Rust toolchain, podman, kubectl, builds from"
            echo "source, installs binary, sets up systemd service on port 5090."
            exit 0
            ;;
        *)  POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-root}}"
PASS="${POSITIONAL[2]:-${DEPLOY_PASS:-}}"

[ -z "$HOST" ] && error "Usage: $0 <host> [user] [password] [--quick]"

# Determine remote home directory based on login user
if [ "$USER" = "root" ]; then
    REMOTE_HOME="/root"
else
    REMOTE_HOME="/home/${USER}"
fi
REMOTE_DIR="${DEPLOY_DIR:-${REMOTE_HOME}/orchestr8}"

# Use sudo when not deploying as root
SUDO=""
[ "$USER" != "root" ] && SUDO="sudo"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

[ -f "$REPO_DIR/Cargo.toml" ] || error "Not in orchestr8 repo: $REPO_DIR"

# ── SSH/rsync wrappers (SSHPASS env var — no password in ps) ──
_ssh() {
    if [ -n "$PASS" ]; then
        SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
    else
        ssh -o StrictHostKeyChecking=no "${USER}@${HOST}" "$@"
    fi
}

_scp() {
    if [ -n "$PASS" ]; then
        SSHPASS="$PASS" sshpass -e scp -o StrictHostKeyChecking=no "$@"
    else
        scp -o StrictHostKeyChecking=no "$@"
    fi
}

_rsync() {
    local ssh_cmd="ssh -o StrictHostKeyChecking=no"
    if [ -n "$PASS" ]; then
        ssh_cmd="sshpass -e $ssh_cmd"
    fi
    SSHPASS="$PASS" rsync -avz \
        --exclude='.git' --exclude='target/' \
        --exclude='*.o' --exclude='*.d' --exclude='*.rlib' \
        --exclude='*.rmeta' --exclude='*.fingerprint' \
        -e "$ssh_cmd" \
        "$@"
}

# ── Preflight ──
if [ -n "$PASS" ] && ! command -v sshpass &>/dev/null; then
    error "sshpass required for password auth. Install: dnf install sshpass"
fi

# ── Uninstall mode ──
if $UNINSTALL_MODE; then
    echo ""
    echo "  ╔══════════════════════════════════════════════════════╗"
    echo "  ║     🗑️  Orchestr8 Remote Uninstall                   ║"
    echo "  ╚══════════════════════════════════════════════════════╝"
    echo ""
    echo "  Host: ${USER}@${HOST}"
    echo ""

    step "Uninstalling orchestr8"
    _ssh "
        $SUDO systemctl stop orchestr8.service 2>/dev/null || true
        $SUDO systemctl disable orchestr8.service 2>/dev/null || true
        $SUDO rm -f /etc/systemd/system/orchestr8.service
        $SUDO systemctl daemon-reload
        $SUDO rm -f /usr/local/bin/orchestr8
        rm -rf $REMOTE_DIR
        rm -rf ${REMOTE_HOME}/.orchestr8
        echo 'Done'
    " 2>&1
    info "orchestr8 removed from ${HOST}"
    echo ""
    echo "  📁 Kept: Rust toolchain (~/.cargo, ~/.rustup)"
    echo "  📁 Kept: system packages (podman, kubectl)"
    echo ""
    exit 0
fi

TOTAL_STEPS=7
$QUICK_MODE && TOTAL_STEPS=4

echo ""
echo "  ╔══════════════════════════════════════════════════════╗"
echo "  ║     🚀 Orchestr8 Remote Deployment                   ║"
echo "  ║     Universal Runtime Control Plane                  ║"
echo "  ╚══════════════════════════════════════════════════════╝"
echo ""
echo "  Host:     ${USER}@${HOST}"
echo "  Auth:     $([ -n "$PASS" ] && echo "🔑 password" || echo "🔐 SSH key")"
echo "  Local:    $REPO_DIR"
echo "  Remote:   $REMOTE_DIR"
echo "  Mode:     $($QUICK_MODE && echo "⚡ quick (rsync + build only)" || echo "📦 full (deps + rust + build + service)")"
echo "  Port:     5090"
echo ""

# ── Step 1: Rsync repo ──
step "Step 1/${TOTAL_STEPS}: 📤 Syncing repository to ${HOST}"
_rsync "$REPO_DIR/" "${USER}@${HOST}:${REMOTE_DIR}/" 2>&1 | tail -3
info "Synced to ${HOST}:${REMOTE_DIR}"

if ! $QUICK_MODE; then
    # ── Step 2: Install system dependencies ──
    step "Step 2/${TOTAL_STEPS}: 📦 Installing system dependencies"
    _ssh "
        # Detect package manager
        if command -v dnf &>/dev/null; then
            $SUDO dnf install -y gcc make openssl-devel pkg-config podman 2>&1 | tail -3
        elif command -v apt-get &>/dev/null; then
            $SUDO apt-get update -qq
            $SUDO apt-get install -y build-essential libssl-dev pkg-config podman 2>&1 | tail -3
        elif command -v zypper &>/dev/null; then
            $SUDO zypper install -y gcc make libopenssl-devel pkg-config podman 2>&1 | tail -3
        else
            echo 'Unknown package manager — install gcc, openssl-devel, pkg-config manually'
        fi
    " 2>&1
    info "System dependencies installed"

    # ── Step 3: Install Rust toolchain ──
    step "Step 3/${TOTAL_STEPS}: 🦀 Installing Rust toolchain"
    _ssh "
        if command -v rustc &>/dev/null; then
            echo \"Rust already installed: \$(rustc --version)\"
            # Update to latest
            rustup update stable 2>&1 | tail -2
        else
            echo 'Installing Rust via rustup...'
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 2>&1 | tail -3
            source ${REMOTE_HOME}/.cargo/env
            echo \"Installed: \$(rustc --version)\"
        fi
    " 2>&1
    info "Rust toolchain ready"

    # ── Step 4: Build release binary ──
    step "Step 4/${TOTAL_STEPS}: 🔨 Building release binary (this takes ~90s)"
    _ssh "
        source ${REMOTE_HOME}/.cargo/env 2>/dev/null || true
        cd $REMOTE_DIR
        cargo build --release 2>&1 | tail -5
    " 2>&1
    info "Release binary built"

    # ── Step 5: Install binary ──
    step "Step 5/${TOTAL_STEPS}: 📥 Installing to /usr/local/bin"
    _ssh "
        $SUDO install -m 0755 $REMOTE_DIR/target/release/orchestr8 /usr/local/bin/orchestr8
        echo \"Installed: \$(orchestr8 --version)\"
        orchestr8 init --yes 2>&1 || true
    " 2>&1
    info "Binary installed"

    # ── Step 6: Set up systemd service ──
    step "Step 6/${TOTAL_STEPS}: ⚙️  Setting up systemd service (port 5090)"
    _ssh "
        $SUDO tee /etc/systemd/system/orchestr8.service > /dev/null << 'SVCEOF'
[Unit]
Description=Orchestr8 Universal Runtime Control Plane
Documentation=https://github.com/ssahani/orchestr8
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/orchestr8 serve --host 0.0.0.0 --port 5090
Restart=on-failure
RestartSec=5s
WorkingDirectory=${REMOTE_HOME}
Environment=ORCHESTR8_SECRET_KEY=orchestr8-production-key

[Install]
WantedBy=multi-user.target
SVCEOF

        $SUDO systemctl daemon-reload
        $SUDO systemctl enable orchestr8.service 2>/dev/null
        $SUDO systemctl restart orchestr8.service
        sleep 2

        if $SUDO systemctl is-active orchestr8 &>/dev/null; then
            echo 'orchestr8 service: running'
        else
            echo 'orchestr8 service: FAILED TO START'
            $SUDO journalctl -u orchestr8 --no-pager -n 5
        fi
    " 2>&1
    info "Systemd service configured"

    # ── Step 7: Open firewall ──
    step "Step 7/${TOTAL_STEPS}: 🔥 Configuring firewall"
    _ssh "
        if command -v firewall-cmd &>/dev/null; then
            $SUDO firewall-cmd --add-port=5090/tcp --permanent 2>/dev/null || true
            $SUDO firewall-cmd --reload 2>/dev/null || true
            echo 'Firewall: port 5090/tcp opened'
        else
            echo 'No firewall-cmd found — skipping'
        fi
    " 2>&1
    info "Firewall configured"
else
    # ── Quick mode: rsync + build + install ──
    step "Step 2/${TOTAL_STEPS}: 🔨 Building release binary"
    _ssh "
        source ${REMOTE_HOME}/.cargo/env 2>/dev/null || true
        cd $REMOTE_DIR
        cargo build --release 2>&1 | tail -5
    " 2>&1
    info "Release binary built"

    step "Step 3/${TOTAL_STEPS}: 📥 Installing to /usr/local/bin"
    _ssh "
        $SUDO install -m 0755 $REMOTE_DIR/target/release/orchestr8 /usr/local/bin/orchestr8
        echo \"Installed: \$(orchestr8 --version)\"
    " 2>&1
    info "Binary installed"

    step "Step 4/${TOTAL_STEPS}: 🔄 Restarting service"
    _ssh "
        $SUDO systemctl restart orchestr8.service 2>/dev/null || true
        sleep 1
        if $SUDO systemctl is-active orchestr8 &>/dev/null; then
            echo 'orchestr8 service: running'
        else
            echo 'orchestr8 service: not running (start with: sudo systemctl start orchestr8)'
        fi
    " 2>&1
    info "Service restarted"
fi

# ── Verify ──
step "Verifying installation"
_ssh "
    echo \"📍 binary:   \$(which orchestr8 2>/dev/null || echo NOT_FOUND)\"
    echo \"📍 version:  \$(orchestr8 --version 2>/dev/null || echo FAILED)\"

    # Check runtimes
    for tool in podman kubectl virtctl; do
        if command -v \$tool &>/dev/null; then
            echo \"📍 \$tool: \$(command -v \$tool)\"
        else
            echo \"⚠️  \$tool: not found\"
        fi
    done

    # Check service
    state=\$(systemctl is-active orchestr8 2>/dev/null || echo 'not-found')
    echo \"📍 service:  \$state\"

    # Check port
    if ss -tlnp 2>/dev/null | grep -q ':5090'; then
        echo \"📍 port 5090: listening\"
    else
        echo \"⚠️  port 5090: not listening\"
    fi

    # Check state dir
    [ -d ${REMOTE_HOME}/.orchestr8 ] && echo '📍 state dir: OK' || echo '⚠️  state dir: missing'
" 2>&1

# ── Smoke test ──
step "Smoke testing endpoints"
_ssh "
    pass=0; fail=0
    test_ep() {
        local label=\"\$1\" url=\"\$2\" expect=\"\$3\"
        code=\$(curl -sk -o /dev/null -w '%{http_code}' --max-time 5 \"\$url\" 2>/dev/null)
        if [ \"\$code\" = \"\$expect\" ]; then
            printf '  ✅ %-20s %s -> %s\n' \"\$label\" \"\$url\" \"\$code\"
            pass=\$((pass + 1))
        else
            printf '  ❌ %-20s %s -> %s (expected %s)\n' \"\$label\" \"\$url\" \"\$code\" \"\$expect\"
            fail=\$((fail + 1))
        fi
    }

    echo ''
    test_ep 'Dashboard'   'http://localhost:5090/'               200
    test_ep 'Health'      'http://localhost:5090/health'          200
    test_ep 'API'         'http://localhost:5090/api/workloads'   200
    test_ep 'Metrics'     'http://localhost:5090/api/metrics'     200
    echo ''
    echo \"  Results: \${pass} passed, \${fail} failed\"
" 2>&1

echo ""
echo "  ════════════════════════════════════════════════════════"
echo "  🎉 Deployment complete: ${USER}@${HOST}"
echo "  ════════════════════════════════════════════════════════"
echo ""
echo "  🔗 SSH:"
echo "    ssh ${USER}@${HOST}"
echo ""
echo "  🌐 Web Dashboard:"
echo "    http://${HOST}:5090"
echo "    Login: admin / orchestr8"
echo ""
echo "  🚀 CLI commands:"
echo "    orchestr8 init              # Setup wizard"
echo "    orchestr8 run --spec app.yaml --runtime podman"
echo "    orchestr8 tui               # Interactive dashboard"
echo "    orchestr8 help-all          # Full command reference"
echo ""
echo "  🔄 Redeploy (quick):"
echo "    $0 ${HOST} ${USER} ${PASS:+***} --quick"
echo ""
