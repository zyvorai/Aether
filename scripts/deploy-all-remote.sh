#!/usr/bin/env bash
# Convenience alias for full remote deploy orchestration.
# Example:
#   ./scripts/deploy-all-remote.sh --remote 212.8.252.194 sus --with-observability
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"
aether_sparkle_line "Remote relay"
echo -e "${A_CYN}${A_BLD}     🛰️  Handing off to deploy-all.sh…${A_RST}"
echo ""
exec "${SCRIPT_DIR}/deploy-all.sh" "$@"
