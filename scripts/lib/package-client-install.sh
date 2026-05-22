#!/usr/bin/env bash
set -euo pipefail
echo "== Aether client dependencies =="
if command -v kubectl &>/dev/null; then echo "  kubectl: OK"
else
  command -v dnf &>/dev/null && sudo dnf install -y kubectl 2>/dev/null || true
  command -v apt-get &>/dev/null && sudo apt-get install -y kubectl 2>/dev/null || true
fi
echo "  Cluster: Kubernetes (Podman/KubeVirt/Metal3 optional per workload)"
echo "Done."
