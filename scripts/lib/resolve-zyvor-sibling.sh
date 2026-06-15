#!/usr/bin/env bash
# shellcheck shell=bash
# Resolve sibling Zyvor product checkouts (tt/VMRogue, tt/v9s, tt/Aether, …).
#
# Usage (from another script):
#   # shellcheck source=lib/resolve-zyvor-sibling.sh
#   source "${SCRIPT_DIR}/lib/resolve-zyvor-sibling.sh"
#   VMROGUE_ROOT="$(resolve_zyvor_sibling "${REPO_ROOT}" VMRogue)" || die "VMRogue not found"

zyvor_repo_root_from() {
  local anchor="${1:?}"
  cd "$(dirname "${anchor}")/.." && pwd
}

zyvor_tt_root_from() {
  local anchor="${1:?}"
  cd "$(zyvor_repo_root_from "${anchor}")/.." && pwd
}

resolve_zyvor_sibling() {
  local repo_root="${1:?}"
  local name="${2:?}"
  local env_key
  env_key="$(printf '%s_REPO' "$(echo "${name}" | tr '[:lower:]-' '[:upper:]_')")"
  local override="${!env_key:-}"
  local tt lower upper
  tt="$(cd "${repo_root}/.." && pwd)"
  lower="$(echo "${name}" | tr '[:upper:]' '[:lower:]')"
  upper="$(echo "${name:0:1}" | tr '[:lower:]' '[:upper:]')${name:1}"

  local d
  for d in \
    "${override}" \
    "${repo_root}/../${name}" \
    "${repo_root}/../${lower}" \
    "${repo_root}/../${upper}" \
    "${tt}/${name}" \
    "${tt}/${lower}" \
    "${tt}/${upper}"; do
    [[ -n "${d}" && -d "${d}" ]] || continue
    printf '%s\n' "${d}"
    return 0
  done
  return 1
}

resolve_zyvor_script() {
  local repo_root="${1:?}"
  local sibling="${2:?}"
  local rel="${3:?}"
  local root
  root="$(resolve_zyvor_sibling "${repo_root}" "${sibling}")" || return 1
  local path="${root}/${rel}"
  [[ -f "${path}" ]] || return 1
  printf '%s\n' "${path}"
}
