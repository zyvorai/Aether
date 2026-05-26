#!/usr/bin/env bash
# shellcheck shell=bash
# Fancy (but optional) terminal output for Aether deploy scripts.
# Disable with: NO_COLOR=1, CI=true, AETHER_PLAIN_DEPLOY=1, or non-TTY stdout.

_aether_tty_color() {
  [[ -z "${NO_COLOR:-}" && -z "${AETHER_PLAIN_DEPLOY:-}" && "${CI:-}" != "true" && -t 1 ]]
}

if _aether_tty_color; then
  A_OR=$'\033[38;5;208m'   # Aether orange
  A_OR2=$'\033[38;5;214m'  # lighter orange
  A_DIM=$'\033[2m'
  A_GRN=$'\033[0;32m'
  A_GRN2=$'\033[38;5;82m'
  A_YLW=$'\033[0;33m'
  A_RED=$'\033[0;31m'
  A_CYN=$'\033[0;36m'
  A_BLU=$'\033[38;5;39m'
  A_MAG=$'\033[0;35m'
  A_WHT=$'\033[1;37m'
  A_BLD=$'\033[1m'
  A_RST=$'\033[0m'
else
  A_OR='' A_OR2='' A_DIM='' A_GRN='' A_GRN2='' A_YLW='' A_RED=''
  A_CYN='' A_BLU='' A_MAG='' A_WHT='' A_BLD='' A_RST=''
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

aether_banner_remote() {
  echo ""
  echo -e "${A_OR}${A_BLD}"
  cat <<'BANNER'
     🚀 ╭──────────────────────────────────────────╮
        │   ░█▀█░█▀▀░█▀▀░█░█░█▀▄░   REMOTE DEPLOY   │
        │   ░█▀█░█▀▀░▀▀█░█▀█░█▀▄░   ─────────────   │
        │   ░▀░▀░▀▀▀░▀▀▀░▀░▀░▀░▀░   Control Plane   │
        ╰──────────────────────────────────────────╯
BANNER
  echo -e "${A_RST}"
  echo -e "${A_MAG}     🌐  SSH  →  🦀 Rust  →  🐳 Image  →  ☸️  Cluster${A_RST}"
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

aether_kv_icon() {
  local icon="$1" key="$2" val="$3"
  printf "  ${icon}  ${A_CYN}%-14s${A_RST} ${A_WHT}%s${A_RST}\n" "${key}" "${val}"
}

aether_progress_bar() {
  local current="${1:-0}" total="${2:-5}" width="${3:-24}"
  if ! _aether_tty_color; then
    printf "  [%s/%s]\n" "${current}" "${total}"
    return
  fi
  local filled=$((current * width / total))
  local empty=$((width - filled))
  local bar=""
  local i
  for ((i = 0; i < filled; i++)); do bar+="█"; done
  for ((i = 0; i < empty; i++)); do bar+="░"; done
  echo -e "  ${A_OR2}${bar}${A_RST} ${A_DIM}${current}/${total}${A_RST}"
}

aether_step() {
  echo ""
  echo -e "  ${A_OR}${A_BLD}▶${A_RST} ${A_BLD}$*${A_RST}"
  echo -e "  ${A_DIM}─────────────────────────────────────────────────${A_RST}"
}

aether_step_remote() {
  local n="$1" total="$2" emoji="$3"
  shift 3
  echo ""
  aether_progress_bar "${n}" "${total}"
  echo -e "  ${emoji}  ${A_OR}${A_BLD}Step ${n}/${total}${A_RST}  ${A_BLD}$*${A_RST}"
  echo -e "  ${A_DIM}─────────────────────────────────────────────────${A_RST}"
}

aether_ok() { echo -e "  ${A_GRN2}✓${A_RST} $*"; }
aether_warn() { echo -e "  ${A_YLW}⚡${A_RST} $*" >&2; }
aether_bad() { echo -e "  ${A_RED}✗${A_RST} $*" >&2; }
aether_die() { aether_bad "$@"; exit 1; }

aether_hint() { echo -e "  ${A_BLU}💡${A_RST} ${A_DIM}$*${A_RST}"; }

aether_url_box() {
  local label="$1" url="$2"
  echo ""
  echo -e "  ${A_GRN2}${A_BLD}╭─ ${label} ─────────────────────────────────${A_RST}"
  echo -e "  ${A_GRN2}${A_BLD}│${A_RST}  ${A_OR2}${A_BLD}🔗  ${url}${A_RST}"
  echo -e "  ${A_GRN2}${A_BLD}╰────────────────────────────────────────────────${A_RST}"
}

aether_finale_success() {
  echo ""
  echo -e "  ${A_GRN}${A_BLD}✨  Mission accomplished${A_RST}  ${A_DIM}— manifests applied, spirits aligned.${A_RST}"
  echo -e "  ${A_OR}🜂${A_RST}  ${A_DIM}May your rollouts be boring and your clusters calm.${A_RST}"
  echo ""
}

aether_finale_remote_deploy() {
  local url="${1:-}"
  local health="${2:-}"
  local elapsed="${3:-}"
  echo ""
  echo -e "  ${A_GRN2}${A_BLD}🎉  DEPLOY COMPLETE${A_RST}  ${A_DIM}— Aether is live on the cluster.${A_RST}"
  if [ -n "${elapsed}" ]; then
    echo -e "  ${A_DIM}⏱  Elapsed: ${elapsed}s${A_RST}"
  fi
  if [ -n "${url}" ]; then
    aether_url_box "Dashboard" "${url}"
  fi
  if [ -n "${health}" ] && [ "${health}" != "${url}" ]; then
    aether_url_box "Health check" "${health}"
  fi
  echo -e "  ${A_MAG}🧪${A_RST}  ${A_DIM}Next: ${A_RST}${A_CYN}./scripts/remote-api-ux-verify.sh${A_RST} ${A_DIM}·${A_RST} ${A_CYN}./scripts/health-check-all.sh${A_RST}"
  echo -e "  ${A_OR}🚀${A_RST}  ${A_DIM}May your NodePorts be open and your rollouts serene.${A_RST}"
  echo ""
}

aether_finale_uninstall() {
  echo ""
  echo -e "  ${A_YLW}${A_BLD}🌑  Uninstall complete${A_RST}  ${A_DIM}— resources returned to the void.${A_RST}"
  echo -e "  ${A_DIM}👋  Until next time, operator.${A_RST}"
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
