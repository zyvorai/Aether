#!/usr/bin/env bash
# Deploy Aether with Stripe production billing env (see examples/deploy/stripe-production.env.example).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${AETHER_DEPLOY_ENV:-$ROOT/examples/deploy/stripe-production.env.example}"
if [ -f "${ENV_FILE}" ]; then
  set -a
  # shellcheck disable=SC1090
  source "${ENV_FILE}"
  set +a
fi
if [ -z "${AETHER_STRIPE_SECRET_KEY:-}" ]; then
  echo "Set AETHER_STRIPE_SECRET_KEY in ${ENV_FILE} or the environment" >&2
  exit 1
fi
if [[ "${AETHER_STRIPE_SECRET_KEY}" != sk_live_* ]]; then
  echo "Warning: AETHER_STRIPE_SECRET_KEY does not look like a live key (sk_live_*)" >&2
fi
export AETHER_HOSTED_SAAS=1
exec "${ROOT}/scripts/deploy-remote.sh" "$@"
