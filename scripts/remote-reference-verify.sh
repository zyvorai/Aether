#!/usr/bin/env bash
# Remote reference cluster verification: k8s live labs on SSH host + post-deploy API checks.
#
# Usage:
#   ./scripts/remote-reference-verify.sh
#   AETHER_REMOTE_HOST=212.8.252.194 AETHER_REMOTE_USER=sus ./scripts/remote-reference-verify.sh
#   AETHER_REMOTE_SKIP_LABS=1 ./scripts/remote-reference-verify.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

HOST="${AETHER_REMOTE_HOST:-212.8.252.194}"
USER="${AETHER_REMOTE_USER:-sus}"
API="${AETHER_API:-http://${HOST}:30090}"
REMOTE_DIR="${AETHER_REMOTE_DIR:-~/.deployment/aether}"
SSH=(ssh -o BatchMode=yes "${USER}@${HOST}")

echo "==> Remote reference verify → ${USER}@${HOST} (${API})"

if [ "${AETHER_REMOTE_SKIP_LABS:-0}" != "1" ]; then
  echo "==> Sync lab scripts to remote"
  rsync -az "${ROOT}/scripts/k8s-labs-e2e.sh" "${ROOT}/scripts/labs-e2e.sh" \
    "${USER}@${HOST}:.deployment/aether/scripts/"
  echo "==> Live Kubernetes lab on remote (AETHER_LABS_LIVE=1)"
  "${SSH[@]}" "cd ${REMOTE_DIR} && chmod +x scripts/k8s-labs-e2e.sh && AETHER_LABS_LIVE=1 AETHER_BIN=./target/release/aether ./scripts/k8s-labs-e2e.sh"
else
  echo "  (skip remote k8s labs: AETHER_REMOTE_SKIP_LABS=1)"
fi

echo "==> Post-deploy API verify → ${API}"
AETHER_API="${API}" "${ROOT}/scripts/post-deploy-verify.sh"

echo "==> Remote reference verify passed"
