#!/usr/bin/env bash
# ============================================================================
# install-confidential-fabric.sh — Full cluster prerequisites (Kata + SPIRE)
#
# Usage:
#   ./scripts/install-confidential-fabric.sh
#   ./scripts/install-confidential-fabric.sh --skip-kata --skip-spire
#   ./scripts/install-confidential-fabric.sh --enable-backend
#
# Options:
#   --skip-kata           Skip CoCo / Zyvor RuntimeClasses
#   --skip-spire          Skip SPIRE Helm install
#   --enable-backend      Run enable-confidential-production.sh after install
#   --gpu                 Pass --gpu to kata installer
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

SKIP_KATA=false
SKIP_SPIRE=false
ENABLE_BACKEND=false
KATA_EXTRA=()

for arg in "$@"; do
  case "$arg" in
    --skip-kata) SKIP_KATA=true ;;
    --skip-spire) SKIP_SPIRE=true ;;
    --enable-backend) ENABLE_BACKEND=true ;;
    --gpu) KATA_EXTRA+=(--gpu) ;;
    -h|--help)
      sed -n '2,18p' "$0"
      exit 0
      ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done

if ! $SKIP_KATA; then
  "${SCRIPT_DIR}/install-confidential-kata.sh" "${KATA_EXTRA[@]}"
fi

if ! $SKIP_SPIRE; then
  "${SCRIPT_DIR}/install-confidential-spire.sh"
fi

if $ENABLE_BACKEND; then
  "${SCRIPT_DIR}/enable-confidential-production.sh" --spire
fi

echo "[confidential-fabric] Cluster fabric prerequisites complete."
