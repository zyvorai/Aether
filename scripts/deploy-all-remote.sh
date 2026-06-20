#!/usr/bin/env bash
# Full remote deploy orchestration (rsync + build + image + cluster + optional observability).
#
# Examples:
#   ./scripts/deploy-all-remote.sh 212.8.252.194 sus
#   ./scripts/deploy-all-remote.sh 212.8.252.194 sus --with-observability
#   ./scripts/deploy-all-remote.sh --remote 212.8.252.194 sus --quick
#   make deploy-all-remote
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"

DEFAULT_HOST="${DEPLOY_HOST:-${AETHER_REMOTE_HOST:-}}"
DEFAULT_USER="${DEPLOY_USER:-${AETHER_REMOTE_USER:-}}"

ARGS=()
if [[ $# -eq 0 ]]; then
  ARGS=(--remote "${DEFAULT_HOST}" "${DEFAULT_USER}")
elif [[ "${1}" != --* && "${1}" != "-h" && "${1}" != "--help" ]]; then
  ARGS=(--remote "$@")
else
  ARGS=("$@")
fi

aether_sparkle_line "Remote relay"
echo -e "${A_CYN}${A_BLD}     🛰️  Handing off to deploy-all.sh…${A_RST}"
aether_kv "Target" "${ARGS[*]#--remote }"
echo ""
exec "${SCRIPT_DIR}/deploy-all.sh" "${ARGS[@]}"
