#!/usr/bin/env bash
# ============================================================================
# deploy-composite-fabric-remote.sh — Composite Ragnarok + Aether confidential fabric
#
# Thin wrapper around the Ragnarok driver (Kata, SPIRE, Aether deploy, hub link).
#
# Usage:
#   ./scripts/deploy-composite-fabric-remote.sh 212.8.252.194 sus
#   ./scripts/deploy-composite-fabric-remote.sh 212.8.252.194 sus --skip-kata
#
# Environment:
#   RAGNAROK_REPO   Path to Ragnarok clone (default: ../ragnarok)
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

find_ragnarok_repo() {
  for d in "${RAGNAROK_REPO:-}" "${ROOT}/../ragnarok" "${ROOT}/../Ragnarok"; do
    [ -n "${d}" ] && [ -x "${d}/scripts/deploy-composite-fabric-remote.sh" ] && echo "${d}" && return 0
  done
  return 1
}

RGN="$(find_ragnarok_repo)" || {
  echo "ERROR: Ragnarok repo not found. Clone sibling ragnarok/ or set RAGNAROK_REPO." >&2
  exit 1
}

export AETHER_REPO="${AETHER_REPO:-${ROOT}}"
exec "${RGN}/scripts/deploy-composite-fabric-remote.sh" "$@"
