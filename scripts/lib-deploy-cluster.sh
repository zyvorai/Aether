# shellcheck shell=bash
# Shared cluster bootstrap for deploy-k8s.sh and deploy-remote.sh.
#
# Cilium bootstrap env:
#   AETHER_SKIP_CILIUM_BOOTSTRAP / AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP — skip all Cilium applies
#   AETHER_CILIUM_EGRESS_STRICT=1 — kube-apiserver + kube-dns/CoreDNS only (not toEntities: all)
#   AETHER_CILIUM_STRICT_ALLOW_CLUSTER=1 — with strict mode, also allow toEntities: cluster (add-on policies)

aether_rbac_manifest() {
  local root="${1:?repo root}"
  echo "${root}/deploy/k8s/rbac/aether-rbac.yaml"
}

aether_apply_rbac() {
  local root="${1:?repo root}"
  local ns="${2:?namespace}"
  local kubectl_bin="${3:-kubectl}"
  local rbac
  rbac="$(aether_rbac_manifest "${root}")"
  [ -f "${rbac}" ] || {
    echo "Missing RBAC manifest: ${rbac}" >&2
    return 1
  }
  sed "s/__AETHER_NAMESPACE__/${ns}/g" "${rbac}" | "${kubectl_bin}" apply -f -
}

# Remove Cilium policies this repo manages (safe before re-apply / mode switch).
aether_delete_managed_cilium_policies() {
  local ns="${1:?namespace}"
  local kubectl_bin="${2:-kubectl}"
  if "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    "${kubectl_bin}" delete cnp -n "${ns}" \
      allow-aether-egress \
      allow-aether-egress-strict \
      allow-aether-egress-strict-cluster \
      --ignore-not-found 2>/dev/null || true
  fi
  if "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
    "${kubectl_bin}" delete ccnp \
      aether-control-plane-egress \
      aether-control-plane-egress-strict \
      aether-control-plane-egress-strict-cluster \
      --ignore-not-found 2>/dev/null || true
  fi
}

# Apply Cilium policies when CRDs exist (egress for aether-system pods).
aether_apply_cilium_bootstrap() {
  local root="${1:?repo root}"
  local ns="${2:?namespace}"
  local kubectl_bin="${3:-kubectl}"
  local skip="${AETHER_SKIP_CILIUM_BOOTSTRAP:-${AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP:-}}"
  local strict="${AETHER_CILIUM_EGRESS_STRICT:-}"
  local strict_cluster="${AETHER_CILIUM_STRICT_ALLOW_CLUSTER:-}"

  if [ "${skip}" = "1" ] || [ "${skip}" = "true" ]; then
    echo "Skipping Cilium bootstrap (AETHER_SKIP_CILIUM_BOOTSTRAP / AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP)"
    return 0
  fi

  if ! "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    echo "CiliumNetworkPolicy CRD not found — skipping Cilium bootstrap"
    return 0
  fi

  aether_delete_managed_cilium_policies "${ns}" "${kubectl_bin}"

  local bootstrap="${root}/deploy/k8s/bootstrap"
  if [ ! -d "${bootstrap}" ]; then
    echo "Missing bootstrap directory: ${bootstrap}" >&2
    return 0
  fi

  if [ "${strict}" = "1" ] || [ "${strict}" = "true" ]; then
    local s="${bootstrap}/cilium-aether-egress-strict.yaml"
    [ -f "${s}" ] || {
      echo "Missing ${s}" >&2
      return 1
    }
    sed "s/__AETHER_NAMESPACE__/${ns}/g" "${s}" | "${kubectl_bin}" apply -f -
    echo "Applied CiliumNetworkPolicy allow-aether-egress-strict (${ns}) — kube-apiserver + DNS"

    if [ "${strict_cluster}" = "1" ] || [ "${strict_cluster}" = "true" ]; then
      local sc="${bootstrap}/cilium-aether-egress-strict-cluster.yaml"
      [ -f "${sc}" ] && sed "s/__AETHER_NAMESPACE__/${ns}/g" "${sc}" | "${kubectl_bin}" apply -f - && \
        echo "Applied CiliumNetworkPolicy allow-aether-egress-strict-cluster (${ns})"
    fi

    if [ -f "${bootstrap}/cilium-aether-clusterwide-egress-strict.yaml" ] && \
      "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
      sed "s/__AETHER_NAMESPACE__/${ns}/g" "${bootstrap}/cilium-aether-clusterwide-egress-strict.yaml" | \
        "${kubectl_bin}" apply -f - && echo "Applied CiliumClusterwideNetworkPolicy aether-control-plane-egress-strict" || \
        echo "Optional clusterwide strict Cilium policy not applied"
    fi

    if [ "${strict_cluster}" = "1" ] || [ "${strict_cluster}" = "true" ]; then
      local scc="${bootstrap}/cilium-aether-clusterwide-egress-strict-cluster.yaml"
      if [ -f "${scc}" ] && "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
        sed "s/__AETHER_NAMESPACE__/${ns}/g" "${scc}" | "${kubectl_bin}" apply -f - && \
          echo "Applied CiliumClusterwideNetworkPolicy aether-control-plane-egress-strict-cluster" || true
      fi
    fi
    echo "Strict Cilium egress: no Internet (world) unless you add policies or use permissive mode."
    return 0
  fi

  local egress="${bootstrap}/cilium-aether-egress.yaml"
  if [ ! -f "${egress}" ]; then
    echo "Missing ${egress}" >&2
    return 0
  fi

  sed "s/__AETHER_NAMESPACE__/${ns}/g" "${egress}" | "${kubectl_bin}" apply -f -
  echo "Applied CiliumNetworkPolicy allow-aether-egress (${ns}) — toEntities: all"

  local ccnp="${bootstrap}/cilium-aether-clusterwide-egress.yaml"
  if [ -f "${ccnp}" ] && "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
    sed "s/__AETHER_NAMESPACE__/${ns}/g" "${ccnp}" | "${kubectl_bin}" apply -f - && \
      echo "Applied CiliumClusterwideNetworkPolicy aether-control-plane-egress" || \
      echo "Optional clusterwide Cilium policy not applied"
  fi
}

aether_print_cluster_mesh_report() {
  local kubectl_bin="${1:-kubectl}"
  echo ""
  aether_sparkle_line "Cluster mesh"
  echo -e "${A_CYN}${A_BLD}     🕸  Runtime stack probe${A_RST}"
  echo ""

  if "${kubectl_bin}" get crd virtualmachines.kubevirt.io &>/dev/null; then
    aether_kv "KubeVirt" "${A_GRN}CRDs present${A_RST}"
  else
    aether_kv "KubeVirt" "${A_DIM}— (install KubeVirt for VM workloads)${A_RST}"
  fi

  if "${kubectl_bin}" get crd baremetalhosts.metal3.io &>/dev/null; then
    aether_kv "Metal3" "${A_GRN}BareMetalHost CRD present${A_RST}"
  else
    aether_kv "Metal3" "${A_DIM}— (optional bare metal)${A_RST}"
  fi

  if "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    aether_kv "Cilium" "${A_GRN}Network policies available${A_RST}"
    if [ "${AETHER_CILIUM_EGRESS_STRICT:-}" = "1" ] || [ "${AETHER_CILIUM_EGRESS_STRICT:-}" = "true" ]; then
      if [ "${AETHER_CILIUM_STRICT_ALLOW_CLUSTER:-}" = "1" ] || [ "${AETHER_CILIUM_STRICT_ALLOW_CLUSTER:-}" = "true" ]; then
        aether_kv "Cilium egress" "${A_OR}strict + in-cluster${A_RST}"
      else
        aether_kv "Cilium egress" "${A_OR}strict (API + DNS)${A_RST}"
      fi
    else
      aether_kv "Cilium egress" "permissive (toEntities: all)"
    fi
  else
    aether_kv "Cilium" "${A_DIM}— (Cilium bootstrap skipped)${A_RST}"
  fi

  if "${kubectl_bin}" get crd datavolumes.cdi.kubevirt.io &>/dev/null; then
    aether_kv "CDI" "${A_GRN}DataVolume CRD present${A_RST}"
  else
    aether_kv "CDI" "${A_DIM}— (optional with KubeVirt images)${A_RST}"
  fi

  echo ""
}
