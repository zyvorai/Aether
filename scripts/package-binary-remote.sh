#!/usr/bin/env bash
# package-binary-remote.sh — Build Aether client bundle on remote Linux host
# Usage: ./scripts/package-binary-remote.sh <host> [user] [--fetch] [--reuse-build] [--skip-deps]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

FETCH=false REUSE_BUILD=false SKIP_DEPS=false
POSITIONAL=()
for arg in "$@"; do
    case "$arg" in
        --fetch) FETCH=true ;; --reuse-build) REUSE_BUILD=true ;; --skip-deps) SKIP_DEPS=true ;;
        -h|--help) sed -n '2,5p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) POSITIONAL+=("$arg") ;;
    esac
done

HOST="${POSITIONAL[0]:-${DEPLOY_HOST:-}}"
USER="${POSITIONAL[1]:-${DEPLOY_USER:-sus}}"
SSH_TIMEOUT="${DEPLOY_SSH_TIMEOUT:-20}"
[[ -n "${HOST}" ]] || { echo "Usage: $0 <host> [user] [--fetch]" >&2; exit 1; }

VERSION="${AETHER_PACKAGE_VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "${REPO_DIR}/Cargo.toml" | head -1)}"
VERSION="${VERSION:-0.3.0}"
ARCH="linux-amd64"
REMOTE="${USER}@${HOST}"
REMOTE_HOME=$(ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" "${REMOTE}" 'echo "$HOME"')
BUILD_DIR="${REMOTE_HOME}/.deployment/aether-package"
OUT_DIR="${AETHER_PACKAGE_DIR:-${REMOTE_HOME}/aether-dist}"
ARTIFACT="aether-${VERSION}-${ARCH}"
LOCAL_DIST="${REPO_DIR}/dist"

RSYNC_EXCLUDES=(--exclude='.git/' --exclude='target/' --exclude='web/dashboard/node_modules/')

log() { printf '  %s\n' "$*"; }
step() { echo ""; printf '── %s\n' "$*"; }

[[ "${AETHER_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]] && { step "Preflight: SSH"; ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" "${REMOTE}" "true"; }

step "Sync → ${BUILD_DIR}"
ssh "${REMOTE}" "mkdir -p '${BUILD_DIR}'"
rsync -az --delete "${RSYNC_EXCLUDES[@]}" -e "ssh -o StrictHostKeyChecking=no" "${REPO_DIR}/" "${REMOTE}:${BUILD_DIR}/"

if ! $SKIP_DEPS; then
    step "Install build deps (rust, npm)"
    ssh "${REMOTE}" bash -s <<REMOTE_DEPS
set -euo pipefail
SUDO=""; [ "\$(id -u)" -ne 0 ] && SUDO=sudo
if command -v dnf &>/dev/null; then \$SUDO dnf install -y gcc git openssl-devel pkg-config nodejs npm 2>&1 | tail -6; fi
if ! command -v cargo &>/dev/null; then curl -fsSL https://sh.rustup.rs | sh -s -- -y; fi
source "\$HOME/.cargo/env"
cargo --version && npm --version
REMOTE_DEPS
fi

BUILD_NEEDED=true
$REUSE_BUILD && ssh "${REMOTE}" "test -x '${BUILD_DIR}/target/release/aether'" && BUILD_NEEDED=false && log "Reuse build"

if $BUILD_NEEDED; then
    step "Build (npm + cargo release)"
    ssh "${REMOTE}" bash -s <<REMOTE_BUILD
set -euo pipefail
cd '${BUILD_DIR}'
source "\$HOME/.cargo/env"
cd web/dashboard && (npm ci --prefer-offline 2>/dev/null || npm install) && npm run build
cd '${BUILD_DIR}' && cargo build --release
strip target/release/aether 2>/dev/null || true
REMOTE_BUILD
fi

step "Assemble tarball"
ssh "${REMOTE}" bash -s <<REMOTE_PACK
set -euo pipefail
STAGE='${OUT_DIR}/${ARTIFACT}'
rm -rf "\${STAGE}" && mkdir -p "\${STAGE}"
cp '${BUILD_DIR}/target/release/aether' "\${STAGE}/" && chmod +x "\${STAGE}/aether"
LIB='${BUILD_DIR}/scripts/lib'
cp "\${LIB}/package-install.sh" "\${STAGE}/install.sh"
cp "\${LIB}/package-client-install.sh" "\${STAGE}/install-client-deps.sh"
cp "\${LIB}/package-client-test.sh" "\${STAGE}/test-package.sh"
mkdir -p "\${STAGE}/.package-lib"
cp "\${LIB}/package-uninstall-lib.sh" "\${STAGE}/.package-lib/"
cp "\${LIB}/package-uninstall.sh" "\${STAGE}/uninstall.sh"
chmod +x "\${STAGE}/"install.sh "\${STAGE}/install-client-deps.sh" "\${STAGE}/test-package.sh" "\${STAGE}/uninstall.sh"
cat > "\${STAGE}/aether.env.example" <<'ENV'
# Copy to aether.env
KUBECONFIG=/path/to/kubeconfig.yaml
# AETHER_STATE_DIR=/var/lib/aether
RUST_LOG=info
ENV
cat > "\${STAGE}/QUICKSTART.txt" <<'Q'
1. tar xzf aether-*-linux-amd64.tar.gz && cd aether-*-linux-amd64
2. ./install.sh
3. ./aether serve --host 0.0.0.0 --port 5090
4. https://<host>:5090/web/dashboard/
5. ./test-package.sh
Q
cat > "\${STAGE}/README.txt" <<README
Aether ${VERSION} — client bundle
install.sh, uninstall.sh | aether binary includes embedded dashboard
./aether serve --host 0.0.0.0 --port 5090
README
for req in install.sh uninstall.sh README.txt QUICKSTART.txt aether; do test -e "\${STAGE}/\${req}" || exit 1; done
cd '${OUT_DIR}' && tar czf '${ARTIFACT}.tar.gz' '${ARTIFACT}' && sha256sum '${ARTIFACT}.tar.gz' | tee '${ARTIFACT}.tar.gz.sha256'
REMOTE_PACK

$FETCH && mkdir -p "${LOCAL_DIST}" && scp "${REMOTE}:${OUT_DIR}/${ARTIFACT}.tar.gz" "${REMOTE}:${OUT_DIR}/${ARTIFACT}.tar.gz.sha256" "${LOCAL_DIST}/"
echo "Done: ${OUT_DIR}/${ARTIFACT}.tar.gz"
