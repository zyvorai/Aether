#!/usr/bin/env bash
# ============================================================================
# deploy-all.sh — Orchestrate Aether platform deployment flows
# ============================================================================
# Usage:
#   ./scripts/deploy-all.sh [flags]
#
# Flags:
#   --remote <host> [user]   Deploy Aether to a remote cluster
#   --with-observability     Also deploy Prometheus/Grafana/Loki stack
#   --skip-health            Skip post-deploy health checks
#   --quick                  Passed through to remote deploy flow
#   --local-build            Passed through to remote deploy flow
#   --skip-sync              Passed through to remote deploy flow
#   --skip-cargo             Passed through to remote deploy flow
#   --skip-image             Passed through to remote deploy flow
#   --uninstall              Remove Aether and optional observability stack
# ============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

REMOTE_HOST=""
REMOTE_USER=""
WITH_OBSERVABILITY=0
SKIP_HEALTH=0
UNINSTALL=0
PASSTHROUGH=()

usage() {
  sed -n '2,18p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

info() { echo "  [✓] $*"; }
step() { echo ""; echo "  --- $*"; }
error() { echo "  [✗] $*" >&2; exit 1; }

while [ "$#" -gt 0 ]; do
  case "$1" in
    --remote)
      [ "$#" -ge 2 ] || error "--remote requires <host> [user]"
      REMOTE_HOST="$2"
      shift 2
      if [ "$#" -gt 0 ] && [[ "${1}" != --* ]]; then
        REMOTE_USER="$1"
        shift
      fi
      continue
      ;;
    --with-observability) WITH_OBSERVABILITY=1 ;;
    --skip-health) SKIP_HEALTH=1 ;;
    --uninstall) UNINSTALL=1 ;;
    --quick|--local-build|--skip-sync|--skip-cargo|--skip-image)
      PASSTHROUGH+=("$1")
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      error "Unknown argument: $1"
      ;;
  esac
  shift
done

echo ""
echo "  ============================================"
echo "    Aether Deployment Orchestrator"
echo "  ============================================"
echo ""
echo "  Mode:         $([ -n "${REMOTE_HOST}" ] && echo remote || echo local)"
echo "  Observability:${WITH_OBSERVABILITY}"
echo "  Skip health:  ${SKIP_HEALTH}"
echo "  Uninstall:    ${UNINSTALL}"
echo ""

if [ "${UNINSTALL}" = "1" ]; then
  step "Removing Aether deployment"
  if [ -n "${REMOTE_HOST}" ]; then
    if [ -n "${REMOTE_USER}" ]; then
      "${SCRIPT_DIR}/deploy-remote.sh" --uninstall "${REMOTE_HOST}" "${REMOTE_USER}"
    else
      "${SCRIPT_DIR}/deploy-remote.sh" --uninstall "${REMOTE_HOST}"
    fi
  else
    "${SCRIPT_DIR}/deploy-k8s.sh" delete
  fi

  if [ "${WITH_OBSERVABILITY}" = "1" ]; then
    step "Removing observability stack"
    "${SCRIPT_DIR}/deploy-observability.sh" delete
  fi

  info "Uninstall completed"
  exit 0
fi

step "Deploying Aether"
if [ -n "${REMOTE_HOST}" ]; then
  if [ -n "${REMOTE_USER}" ]; then
    if [ "${#PASSTHROUGH[@]}" -gt 0 ]; then
      "${SCRIPT_DIR}/deploy-remote.sh" "${PASSTHROUGH[@]}" "${REMOTE_HOST}" "${REMOTE_USER}"
    else
      "${SCRIPT_DIR}/deploy-remote.sh" "${REMOTE_HOST}" "${REMOTE_USER}"
    fi
  else
    if [ "${#PASSTHROUGH[@]}" -gt 0 ]; then
      "${SCRIPT_DIR}/deploy-remote.sh" "${PASSTHROUGH[@]}" "${REMOTE_HOST}"
    else
      "${SCRIPT_DIR}/deploy-remote.sh" "${REMOTE_HOST}"
    fi
  fi
else
  "${SCRIPT_DIR}/deploy-k8s.sh" all
fi

if [ "${WITH_OBSERVABILITY}" = "1" ]; then
  step "Deploying observability stack"
  "${SCRIPT_DIR}/deploy-observability.sh" deploy
fi

if [ "${SKIP_HEALTH}" = "0" ]; then
  step "Running health checks"
  "${SCRIPT_DIR}/health-check-all.sh"
else
  info "Skipped health checks"
fi

info "Deployment flow completed"
