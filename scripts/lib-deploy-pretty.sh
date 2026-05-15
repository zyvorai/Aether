#!/usr/bin/env bash
# shellcheck shell=bash
# Fancy (but optional) terminal output for Aether deploy scripts.
# Disable with: NO_COLOR=1, CI=true, AETHER_PLAIN_DEPLOY=1, or non-TTY stdout.

_aether_tty_color() {
  [[ -z "${NO_COLOR:-}" && -z "${AETHER_PLAIN_DEPLOY:-}" && "${CI:-}" != "true" && -t 1 ]]
}

if _aether_tty_color; then
  A_OR=$'\033[38;5;208m' # Aether-ish orange
  A_DIM=$'\033[2m'
  A_GRN=$'\033[0;32m'
  A_YLW=$'\033[0;33m'
  A_RED=$'\033[0;31m'
  A_CYN=$'\033[0;36m'
  A_MAG=$'\033[0;35m'
  A_BLD=$'\033[1m'
  A_RST=$'\033[0m'
else
  A_OR='' A_DIM='' A_GRN='' A_YLW='' A_RED='' A_CYN='' A_MAG='' A_BLD='' A_RST=''
fi

aether_sparkle_line() {
  local title="${1:-Aether}"
  echo -e "${A_OR}${A_BLD}  ✦ · ✧ · ✦  ${title}  ✦ · ✧ · ✦${A_RST}"
}

aether_banner_orchestrator() {
  echo ""
  aether_sparkle_line "Control Plane"
  echo -e "${A_CYN}${A_BLD}     🎼  Deployment Orchestra${A_RST}"
  echo -e "${A_MAG}     ✶  Runtime mesh: Podman · K8s · KubeVirt · Metal3${A_RST}"
  echo -e "${A_DIM}     ─────────────────────────────────────────────────${A_RST}"
}

aether_banner_health() {
  echo ""
  aether_sparkle_line "Health Ritual"
  echo -e "${A_CYN}${A_BLD}     🩺  Stethoscope pass${A_RST}"
  echo -e "${A_DIM}     ─────────────────────────────────────────────────${A_RST}"
}

aether_kv() {
  printf "  ${A_CYN}%-16s${A_RST} %s\n" "$1" "$2"
}

aether_step() {
  echo ""
  echo -e "  ${A_OR}${A_BLD}▶${A_RST} ${A_BLD}$*${A_RST}"
  echo -e "  ${A_DIM}─────────────────────────────────────────────────${A_RST}"
}

aether_ok() { echo -e "  ${A_GRN}✓${A_RST} $*"; }
aether_warn() { echo -e "  ${A_YLW}⚡${A_RST} $*" >&2; }
aether_bad() { echo -e "  ${A_RED}✗${A_RST} $*" >&2; }
aether_die() { aether_bad "$@"; exit 1; }

aether_finale_success() {
  echo ""
  echo -e "  ${A_GRN}${A_BLD}✨  Mission accomplished${A_RST}  ${A_DIM}— manifests applied, spirits aligned.${A_RST}"
  echo -e "  ${A_OR}🜂${A_RST}  ${A_DIM}May your rollouts be boring and your clusters calm.${A_RST}"
  echo ""
}

aether_finale_uninstall() {
  echo ""
  echo -e "  ${A_YLW}${A_BLD}🌑  Uninstall complete${A_RST}  ${A_DIM}— resources returned to the void.${A_RST}"
  echo ""
}

aether_finale_health_ok() {
  echo -e "  ${A_GRN}${A_BLD}✨  All seals intact.${A_RST}  ${A_DIM}No failed checks.${A_RST}"
  echo ""
}

aether_finale_health_bad() {
  echo -e "  ${A_RED}${A_BLD}💀  The ritual found cracks.${A_RST}  ${A_DIM}Fix failures above and re-run.${A_RST}"
  echo ""
}

aether_section() {
  echo ""
  echo -e "  ${A_OR}${A_BLD}◇${A_RST} ${A_BLD}$*${A_RST}"
}

aether_subtle() { echo -e "${A_DIM}$*${A_RST}"; }
