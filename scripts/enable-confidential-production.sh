#!/usr/bin/env bash
# ============================================================================
# enable-confidential-production.sh — Turn on SPIRE + trust enforcement on Ragnarok API
#
# Usage:
#   ./scripts/enable-confidential-production.sh --spire
#   ./scripts/enable-confidential-production.sh --spire --trust-enforce
#   ./scripts/enable-confidential-production.sh --spire              # CSI socket (default)
#   ./scripts/enable-confidential-production.sh --spire --mount-agent-socket  # hostPath (privileged PSS)
#
# Environment:
#   RAGNAROK_NAMESPACE     default ragnarok
#   RAGNAROK_DEPLOYMENT      default ragnarok-backend
#   SPIRE_AGENT_SOCKET       default /run/spire/sockets/agent.sock
#   SPIRE_TRUST_DOMAIN       default ragnarok.zyvor.dev
# ============================================================================

set -euo pipefail

RAGNAROK_NAMESPACE="${RAGNAROK_NAMESPACE:-ragnarok}"
DEPLOYMENT="${RAGNAROK_DEPLOYMENT:-ragnarok-backend}"
SPIRE_SOCKET="${SPIRE_AGENT_SOCKET:-}"
SPIRE_TRUST_DOMAIN="${SPIRE_TRUST_DOMAIN:-ragnarok.zyvor.dev}"
CSI_SOCKET_PATH="${RAGNAROK_SPIRE_CSI_SOCKET:-/spiffe-workload-api/spire-agent.sock}"

ENABLE_SPIRE=false
ENABLE_TRUST=false
MOUNT_HOST_SOCKET=false
USE_CSI_SOCKET=true

for arg in "$@"; do
  case "$arg" in
    --spire) ENABLE_SPIRE=true ;;
    --trust-enforce) ENABLE_TRUST=true ;;
    --mount-agent-socket) MOUNT_HOST_SOCKET=true; USE_CSI_SOCKET=false ;;
    --csi-socket) USE_CSI_SOCKET=true ;;
    --no-socket-mount) USE_CSI_SOCKET=false; MOUNT_HOST_SOCKET=false ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done

if ! $ENABLE_SPIRE && ! $ENABLE_TRUST; then
  echo "Specify --spire and/or --trust-enforce" >&2
  exit 1
fi

log() { echo "[enable-confidential] $*"; }

command -v kubectl >/dev/null 2>&1 || { echo "kubectl required" >&2; exit 1; }

find_patch() {
  local name="$1"
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  for candidate in \
    "${SCRIPT_DIR}/deploy/confidential/spire/${name}" \
    "${SCRIPT_DIR}/../deploy/confidential/spire/${name}"; do
    if [ -f "${candidate}" ]; then
      echo "${candidate}"
      return 0
    fi
  done
  return 1
}

ENV_ARGS=()
if $ENABLE_SPIRE; then
  if $USE_CSI_SOCKET && [ -z "${SPIRE_SOCKET}" ]; then
    SPIRE_SOCKET="${CSI_SOCKET_PATH}"
  elif [ -z "${SPIRE_SOCKET}" ]; then
    SPIRE_SOCKET="/run/spire/sockets/agent.sock"
  fi
  ENV_ARGS+=(
    "RAGNAROK_SPIRE_ENABLED=1"
    "SPIRE_AGENT_SOCKET=${SPIRE_SOCKET}"
    "SPIRE_TRUST_DOMAIN=${SPIRE_TRUST_DOMAIN}"
  )
fi
if $ENABLE_TRUST; then
  ENV_ARGS+=("RAGNAROK_TRUST_ENFORCE=1")
fi

log "Patching ${DEPLOYMENT} env in ${RAGNAROK_NAMESPACE}..."
kubectl set env "deployment/${DEPLOYMENT}" -n "${RAGNAROK_NAMESPACE}" "${ENV_ARGS[@]}"

if $ENABLE_SPIRE && $USE_CSI_SOCKET; then
  if PATCH="$(find_patch ragnarok-backend-csi-patch.yaml)"; then
    log "Applying SPIFFE CSI workload API volume (restricted-PSS safe)..."
    kubectl patch "deployment/${DEPLOYMENT}" -n "${RAGNAROK_NAMESPACE}" --patch-file "${PATCH}"
    if CSI_ID="$(find_patch clusterspiffeid-ragnarok-backend.yaml)"; then
      kubectl apply -f "${CSI_ID}" 2>/dev/null || log "WARN: ClusterSPIFFEID apply skipped (may already exist)"
    fi
  else
    log "WARN: missing ragnarok-backend-csi-patch.yaml"
  fi
elif $ENABLE_SPIRE && $MOUNT_HOST_SOCKET; then
  log "Note: hostPath requires non-restricted PSS on ${RAGNAROK_NAMESPACE}"
  if PATCH="$(find_patch ragnarok-backend-socket-patch.yaml)"; then
    kubectl patch "deployment/${DEPLOYMENT}" -n "${RAGNAROK_NAMESPACE}" --patch-file "${PATCH}"
  else
    log "WARN: missing ragnarok-backend-socket-patch.yaml"
  fi
fi

kubectl rollout status "deployment/${DEPLOYMENT}" -n "${RAGNAROK_NAMESPACE}" --timeout=120s
log "Done. Verify: GET /api/v1/confidential/spire/status"
