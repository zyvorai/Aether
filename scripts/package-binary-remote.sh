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

# shellcheck source=lib/package-remote-ui.sh
source "${SCRIPT_DIR}/lib/package-remote-ui.sh"

pkg_remote_banner "Aether" "${VERSION}" "${REMOTE}" "${ARCH}"

if [[ "${AETHER_REMOTE_SKIP_SSH_CHECK:-}" != "1" ]]; then
    pkg_remote_phase "Preflight"
    ssh -o BatchMode=yes -o ConnectTimeout="${SSH_TIMEOUT}" -o StrictHostKeyChecking=accept-new "${REMOTE}" "true"
    pkg_ok "SSH ${REMOTE}"
fi

pkg_remote_phase "Sync source"
pkg_remote_kv "Build dir" "${BUILD_DIR}"
ssh "${REMOTE}" "mkdir -p '${BUILD_DIR}'"
rsync -az --delete "${RSYNC_EXCLUDES[@]}" -e "ssh -o StrictHostKeyChecking=no" "${REPO_DIR}/" "${REMOTE}:${BUILD_DIR}/"

if ! $SKIP_DEPS; then
    pkg_remote_phase "Build dependencies"
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
$REUSE_BUILD && ssh "${REMOTE}" "test -x '${BUILD_DIR}/target/release/aether'" && BUILD_NEEDED=false && pkg_ok "Reuse build"

if $BUILD_NEEDED; then
    pkg_remote_phase "Compile"
    pkg_info "npm dashboard + cargo release…"
    ssh "${REMOTE}" bash -s <<REMOTE_BUILD
set -euo pipefail
cd '${BUILD_DIR}'
source "\$HOME/.cargo/env"
cd web/dashboard && (npm ci --prefer-offline 2>/dev/null || npm install) && npm run build
cd '${BUILD_DIR}' && cargo build --release
strip target/release/aether 2>/dev/null || true
REMOTE_BUILD
fi

pkg_remote_phase "Assemble customer bundle"
pkg_remote_kv "Output" "${OUT_DIR}/${ARTIFACT}"
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
cp "\${LIB}/package-ui.sh" "\${STAGE}/.package-lib/"
cp "\${LIB}/install-everything.sh" "\${STAGE}/"
cp "\${LIB}/package-uninstall-lib.sh" "\${STAGE}/.package-lib/"
cp "\${LIB}/package-uninstall.sh" "\${STAGE}/uninstall.sh"
chmod +x "\${STAGE}/install.sh" "\${STAGE}/install-client-deps.sh" "\${STAGE}/test-package.sh" \
  "\${STAGE}/install-everything.sh" "\${STAGE}/uninstall.sh"
chmod +x "\${LIB}/write-customer-help.sh"
"\${LIB}/write-customer-help.sh" "\${STAGE}" "Aether" k8s
cp "\${LIB}/START_HERE.txt" "\${STAGE}/"
cat > "\${STAGE}/.package-lib/product.meta" <<'META'
PRODUCT_NAME=Aether
ACCESS_SCHEME=https
ACCESS_PORT=5090
ACCESS_PATH=/web/dashboard/
AUTO_FULL_INSTALL=0
FINISH_EXTRA_1='Start: ./aether serve --host 0.0.0.0 --port 5090'
FINISH_EXTRA_2=
FINISH_EXTRA_3=
META
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

Packaged by Zyvor — zyvor.dev · HyperSDK · © 2026
Q

cp '${BUILD_DIR}/scripts/zyvor-branding/ZYVOR_INSTALL.txt' "\${STAGE}/ZYVOR_INSTALL.txt" 2>/dev/null || true

cat > "\${STAGE}/README.txt" <<README
Aether ${VERSION} — client bundle
install.sh, uninstall.sh | aether binary includes embedded dashboard
./aether serve --host 0.0.0.0 --port 5090
README
for req in HELP.txt START_HERE.txt install.sh uninstall.sh README.txt QUICKSTART.txt aether; do test -e "\${STAGE}/\${req}" || exit 1; done
cd '${OUT_DIR}' && tar czf '${ARTIFACT}.tar.gz' '${ARTIFACT}' && sha256sum '${ARTIFACT}.tar.gz' | tee '${ARTIFACT}.tar.gz.sha256'
REMOTE_PACK

TARBALL="${ARTIFACT}.tar.gz"
REMOTE_TARBALL="${OUT_DIR}/${TARBALL}"
if $FETCH; then
    pkg_remote_phase "Fetch to laptop"
    mkdir -p "${LOCAL_DIST}"
    scp -o StrictHostKeyChecking=no \
        "${REMOTE}:${REMOTE_TARBALL}" \
        "${REMOTE}:${OUT_DIR}/${TARBALL}.sha256" \
        "${LOCAL_DIST}/"
    (cd "${LOCAL_DIST}" && shasum -a 256 -c "${TARBALL}.sha256" 2>/dev/null || sha256sum -c "${TARBALL}.sha256") && pkg_ok "Checksum verified"
fi
pkg_remote_done "Aether" "${REMOTE}:${REMOTE_TARBALL}" "${REMOTE}:${OUT_DIR}/${TARBALL}.sha256"
