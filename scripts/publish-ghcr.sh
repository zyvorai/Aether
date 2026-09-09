#!/usr/bin/env bash
# Publish Aether Docker image (+ optional Helm charts) to ghcr.io/zyvorai.
#
# Prefer the Release workflow for images. This script is for manual/lab pushes.
#
# Usage:
#   GHCR_TOKEN=<pat> ./scripts/publish-ghcr.sh [host] [user] [version]
#
# Example:
#   GHCR_TOKEN=ghp_xxx ./scripts/publish-ghcr.sh 212.8.248.187 sus 0.4.0

set -euo pipefail

HOST="${1:-212.8.248.187}"
RUSER="${2:-sus}"
VERSION="${3:-0.4.0}"
REMOTE="${RUSER}@${HOST}"
ORG="zyvorai"
TOKEN="${GHCR_TOKEN:-${HYPERSDK_GHCR_TOKEN:-}}"

if [[ -z "${TOKEN}" ]]; then
  echo "Error: set GHCR_TOKEN (PAT with write:packages) before running" >&2
  echo "  export GHCR_TOKEN='ghp_...'" >&2
  exit 1
fi

echo "==> Publishing aether v${VERSION} to ghcr.io/${ORG}"

echo "==> [1/2] Pushing Docker image to ghcr.io/${ORG}/aether"
ssh "$REMOTE" bash -s <<ENDSSH
set -euo pipefail
echo "${TOKEN}" | podman login ghcr.io -u ${ORG} --password-stdin
podman tag docker.io/library/aether:latest ghcr.io/${ORG}/aether:${VERSION}
podman tag docker.io/library/aether:latest ghcr.io/${ORG}/aether:latest
podman push ghcr.io/${ORG}/aether:${VERSION}
podman push ghcr.io/${ORG}/aether:latest
echo "Image pushed: ghcr.io/${ORG}/aether:${VERSION}"
ENDSSH

echo "==> [2/2] Packaging and pushing Helm charts to oci://ghcr.io/${ORG}/charts"

ssh "$REMOTE" bash -s <<ENDSSH
set -euo pipefail
cd ~/Aether 2>/dev/null || cd ~/aether || cd ~/tt/Aether || true
if [[ ! -d helm/aether ]]; then
  echo "helm/aether not found on remote; skip chart push" >&2
  exit 0
fi
echo "${TOKEN}" | helm registry login ghcr.io -u ${ORG} --password-stdin
helm package helm/aether -d /tmp
tgz=\$(ls -1 /tmp/aether-*.tgz | tail -1)
helm push "\$tgz" oci://ghcr.io/${ORG}/charts
echo "Charts pushed to oci://ghcr.io/${ORG}/charts"
ENDSSH

echo ""
echo "Done."
echo "  docker pull ghcr.io/${ORG}/aether:${VERSION}"
echo "  helm install aether oci://ghcr.io/${ORG}/charts/aether --version ${VERSION}"
