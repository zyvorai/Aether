#!/usr/bin/env bash
# Delegate full Zyvor stack E2E (VMRogue + PacketWolf + Aether) to the VMRogue orchestrator.
#
# Usage:
#   ./scripts/test-zyvor-stack-remote.sh <host> [vmrogue_port] [ssh_user]
#   ./scripts/test-zyvor-stack-remote.sh 212.8.252.194 30151 sus
#
# Environment:
#   VMROGUE_REPO — override sibling path (default: ../VMRogue under tt/)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=lib/resolve-zyvor-sibling.sh
source "${SCRIPT_DIR}/lib/resolve-zyvor-sibling.sh"

STACK_SCRIPT="$(resolve_zyvor_script "${REPO_ROOT}" VMRogue "scripts/test-zyvor-stack-remote.sh")" || {
  echo "ERROR: VMRogue stack orchestrator not found." >&2
  echo "  Clone sibling VMRogue/ next to Aether/ or set VMROGUE_REPO." >&2
  echo "  Expected: $(resolve_zyvor_sibling "${REPO_ROOT}" VMRogue 2>/dev/null || echo '../VMRogue')/scripts/test-zyvor-stack-remote.sh" >&2
  exit 1
}

exec "${STACK_SCRIPT}" "$@"
