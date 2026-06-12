#!/usr/bin/env bash
# Deploy Aether to a remote k8s host with Ingress + TLS (see examples/deploy/reference-ingress.env.example).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${AETHER_DEPLOY_ENV:-$ROOT/examples/deploy/reference-ingress.env.example}"
if [ -f "${ENV_FILE}" ]; then
  set -a
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  set +a
fi
export AETHER_EXPOSE="${AETHER_EXPOSE:-both}"
if [ -z "${AETHER_INGRESS_HOST:-}" ]; then
  echo "Set AETHER_INGRESS_HOST in ${ENV_FILE} or the environment" >&2
  exit 1
fi
exec "${ROOT}/scripts/deploy-remote.sh" "$@"
