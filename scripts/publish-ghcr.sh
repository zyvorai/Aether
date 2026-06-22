#!/usr/bin/env bash
# Publish Aether Docker image + Helm charts to ghcr.io/hypersdk from the remote build server.
#
# Usage:
#   HYPERSDK_GHCR_TOKEN=<pat> ./scripts/publish-ghcr.sh [host] [user] [version]
#
# Example:
#   HYPERSDK_GHCR_TOKEN=ghp_xxx ./scripts/publish-ghcr.sh 212.8.248.187 sus 0.3.0

set -euo pipefail

HOST="${1:-212.8.248.187}"
RUSER="${2:-sus}"
VERSION="${3:-0.3.0}"
REMOTE="${RUSER}@${HOST}"
ORG="hypersdk"

if [[ -z "${HYPERSDK_GHCR_TOKEN:-}" ]]; then
  echo "Error: set HYPERSDK_GHCR_TOKEN before running" >&2
  echo "  export HYPERSDK_GHCR_TOKEN='ghp_...'" >&2
  exit 1
fi

echo "==> Publishing aether v${VERSION} to ghcr.io/${ORG}"

# ── 1. Push Docker image ────────────────────────────────────────────────────
echo "==> [1/2] Pushing Docker image to ghcr.io/${ORG}/aether"
ssh "$REMOTE" bash -s <<ENDSSH
set -euo pipefail
echo "${HYPERSDK_GHCR_TOKEN}" | podman login ghcr.io -u ${ORG} --password-stdin
podman tag docker.io/library/aether:latest ghcr.io/${ORG}/aether:${VERSION}
podman tag docker.io/library/aether:latest ghcr.io/${ORG}/aether:latest
podman push ghcr.io/${ORG}/aether:${VERSION}
podman push ghcr.io/${ORG}/aether:latest
echo "Image pushed: ghcr.io/${ORG}/aether:${VERSION}"
ENDSSH

# ── 2. Push Helm charts ─────────────────────────────────────────────────────
echo "==> [2/2] Packaging and pushing Helm charts to oci://ghcr.io/${ORG}/charts"

CHARTS_DIR="/tmp/aether-charts-${VERSION}"
ssh "$REMOTE" "mkdir -p ${CHARTS_DIR}"
rsync -az --delete charts/ "${REMOTE}:${CHARTS_DIR}/"

ssh "$REMOTE" bash -s <<ENDSSH
set -euo pipefail

if ! command -v helm &>/dev/null; then
  curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
fi

echo "${HYPERSDK_GHCR_TOKEN}" | helm registry login ghcr.io -u ${ORG} --password-stdin

mkdir -p /tmp/helm-packages
for chart in ${CHARTS_DIR}/aether ${CHARTS_DIR}/zyvor-confidential-kata; do
  helm package "\$chart" -d /tmp/helm-packages
done

for tgz in /tmp/helm-packages/*.tgz; do
  echo "Pushing \$tgz"
  helm push "\$tgz" oci://ghcr.io/${ORG}/charts
done

rm -rf /tmp/helm-packages ${CHARTS_DIR}
echo "Charts pushed to oci://ghcr.io/${ORG}/charts"
ENDSSH

echo ""
echo "✓ Done. Customer install:"
echo "  helm install aether oci://ghcr.io/${ORG}/charts/aether --version 0.1.0 \\"
echo "    --namespace aether-system --create-namespace"
