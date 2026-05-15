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
# shellcheck source=lib-deploy-pretty.sh
source "${SCRIPT_DIR}/lib-deploy-pretty.sh"

REMOTE_HOST=""
REMOTE_USER=""
WITH_OBSERVABILITY=0
SKIP_HEALTH=0
UNINSTALL=0
PASSTHROUGH=()
POSITIONAL=()

usage() {
  sed -n '2,18p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

info() { aether_ok "$*"; }
step() { aether_step "$*"; }
error() { aether_die "$*"; }

run_remote_health_checks() {
  local remote_target
  if [ -n "${REMOTE_USER}" ]; then
    remote_target="${REMOTE_USER}@${REMOTE_HOST}"
  else
    remote_target="${REMOTE_HOST}"
  fi

  aether_subtle "  🛰️  Opening SSH channel to ${remote_target} for health ritual…"
  ssh -o StrictHostKeyChecking=no "${remote_target}" \
    "cd ~/.deployment/aether && ./scripts/health-check-all.sh"
}

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
      POSITIONAL+=("$1")
      ;;
  esac
  shift
done

if [ "${#POSITIONAL[@]}" -gt 0 ]; then
  if [ -n "${REMOTE_HOST}" ]; then
    error "Positional remote target cannot be combined with --remote"
  fi

  if [ "${#POSITIONAL[@]}" -gt 2 ]; then
    error "Too many positional arguments"
  fi

  if [[ "${POSITIONAL[0]}" == *@* ]]; then
    REMOTE_USER="${POSITIONAL[0]%%@*}"
    REMOTE_HOST="${POSITIONAL[0]##*@}"
    if [ "${#POSITIONAL[@]}" -gt 1 ]; then
      error "Do not pass a separate user when using user@host syntax"
    fi
  else
    REMOTE_HOST="${POSITIONAL[0]}"
    if [ "${#POSITIONAL[@]}" -gt 1 ]; then
      REMOTE_USER="${POSITIONAL[1]}"
    fi
  fi
fi

aether_banner_orchestrator
aether_kv "Mode" "$([ -n "${REMOTE_HOST}" ] && echo "🌍 remote" || echo "🏠 local")"
aether_kv "Observability" "$([ "${WITH_OBSERVABILITY}" = "1" ] && echo "📊 yes" || echo "—")"
aether_kv "Health checks" "$([ "${SKIP_HEALTH}" = "1" ] && echo "⏭️  skipped" || echo "✅ enabled")"
aether_kv "Uninstall" "$([ "${UNINSTALL}" = "1" ] && echo "🌑 yes" || echo "—")"
echo ""

if [ "${UNINSTALL}" = "1" ]; then
  step "Removing Aether deployment (banishing workloads)"
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
  aether_finale_uninstall
  exit 0
fi

step "Deploying Aether (weaving manifests)"
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
  step "Deploying observability stack (🔭 charts incoming)"
  "${SCRIPT_DIR}/deploy-observability.sh" deploy
fi

if [ "${SKIP_HEALTH}" = "0" ]; then
  step "Running health checks (🩺)"
  if [ -n "${REMOTE_HOST}" ]; then
    run_remote_health_checks
  else
    "${SCRIPT_DIR}/health-check-all.sh"
  fi
else
  info "Skipped health checks (you asked nicely with --skip-health)"
fi

info "Deployment flow completed"
aether_finale_success
