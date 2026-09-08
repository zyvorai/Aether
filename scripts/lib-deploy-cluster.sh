# shellcheck shell=bash
# Shared cluster bootstrap for deploy-k8s.sh and deploy-remote.sh.
#
# Cilium bootstrap env:
#   AETHER_SKIP_CILIUM_BOOTSTRAP / AETHER_SKIP_CILIUM_EGRESS_BOOTSTRAP — skip all Cilium applies
#   AETHER_CILIUM_EGRESS_STRICT=1 — kube-apiserver + kube-dns/CoreDNS only (not toEntities: all)
#   AETHER_CILIUM_STRICT_ALLOW_CLUSTER=1 — with strict mode, also allow toEntities: cluster (add-on policies)

# kubectl may be a multi-word command (e.g. sudo /usr/local/bin/k3s kubectl); do not quote as a single path.
aether__kubectl_exec() {
  local _kubectl="${1:?kubectl command}"
  shift
  # shellcheck disable=SC2086
  ${_kubectl} "$@"
}

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
  sed "s/__AETHER_NAMESPACE__/${ns}/g" "${rbac}" | aether__kubectl_exec "${kubectl_bin}" apply -f -
}

# Remove Cilium policies this repo manages (safe before re-apply / mode switch).
aether_delete_managed_cilium_policies() {
  local ns="${1:?namespace}"
  local kubectl_bin="${2:-kubectl}"
  if aether__kubectl_exec "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    aether__kubectl_exec "${kubectl_bin}" delete cnp -n "${ns}" \
      allow-aether-egress \
      allow-aether-egress-strict \
      allow-aether-egress-strict-cluster \
      --ignore-not-found 2>/dev/null || true
  fi
  if aether__kubectl_exec "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
    aether__kubectl_exec "${kubectl_bin}" delete ccnp \
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

  if ! aether__kubectl_exec "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
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
    sed "s/__AETHER_NAMESPACE__/${ns}/g" "${s}" | aether__kubectl_exec "${kubectl_bin}" apply -f -
    echo "Applied CiliumNetworkPolicy allow-aether-egress-strict (${ns}) — kube-apiserver + DNS"

    if [ "${strict_cluster}" = "1" ] || [ "${strict_cluster}" = "true" ]; then
      local sc="${bootstrap}/cilium-aether-egress-strict-cluster.yaml"
      [ -f "${sc}" ] && sed "s/__AETHER_NAMESPACE__/${ns}/g" "${sc}" | aether__kubectl_exec "${kubectl_bin}" apply -f - && \
        echo "Applied CiliumNetworkPolicy allow-aether-egress-strict-cluster (${ns})"
    fi

    if [ -f "${bootstrap}/cilium-aether-clusterwide-egress-strict.yaml" ] && \
      aether__kubectl_exec "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
      sed "s/__AETHER_NAMESPACE__/${ns}/g" "${bootstrap}/cilium-aether-clusterwide-egress-strict.yaml" | \
        aether__kubectl_exec "${kubectl_bin}" apply -f - && echo "Applied CiliumClusterwideNetworkPolicy aether-control-plane-egress-strict" || \
        echo "Optional clusterwide strict Cilium policy not applied"
    fi

    if [ "${strict_cluster}" = "1" ] || [ "${strict_cluster}" = "true" ]; then
      local scc="${bootstrap}/cilium-aether-clusterwide-egress-strict-cluster.yaml"
      if [ -f "${scc}" ] && aether__kubectl_exec "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
        sed "s/__AETHER_NAMESPACE__/${ns}/g" "${scc}" | aether__kubectl_exec "${kubectl_bin}" apply -f - && \
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

  sed "s/__AETHER_NAMESPACE__/${ns}/g" "${egress}" | aether__kubectl_exec "${kubectl_bin}" apply -f -
  echo "Applied CiliumNetworkPolicy allow-aether-egress (${ns}) — toEntities: all"

  local ccnp="${bootstrap}/cilium-aether-clusterwide-egress.yaml"
  if [ -f "${ccnp}" ] && aether__kubectl_exec "${kubectl_bin}" get crd ciliumclusterwidenetworkpolicies.cilium.io &>/dev/null; then
    sed "s/__AETHER_NAMESPACE__/${ns}/g" "${ccnp}" | aether__kubectl_exec "${kubectl_bin}" apply -f - && \
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

  if aether__kubectl_exec "${kubectl_bin}" get crd virtualmachines.kubevirt.io &>/dev/null; then
    aether_kv "KubeVirt" "${A_GRN}CRDs present${A_RST}"
  else
    aether_kv "KubeVirt" "${A_DIM}— (install KubeVirt for VM workloads)${A_RST}"
  fi

  if aether__kubectl_exec "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
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

  if aether__kubectl_exec "${kubectl_bin}" get crd datavolumes.cdi.kubevirt.io &>/dev/null; then
    aether_kv "CDI" "${A_GRN}DataVolume CRD present${A_RST}"
  else
    aether_kv "CDI" "${A_DIM}— (optional with KubeVirt images)${A_RST}"
  fi

  echo ""
}

# Install metrics-server when missing (auto for k3s/kind/minikube/microk8s).
# Env: AETHER_INSTALL_METRICS_SERVER=auto|1|0 (default auto)
aether_install_metrics_server() {
  local kubectl_bin="${1:-kubectl}"
  local distro="${2:-unknown}"
  local flag="${AETHER_INSTALL_METRICS_SERVER:-auto}"

  case "${flag}" in
    0|false|no)
      echo "Skipping metrics-server (AETHER_INSTALL_METRICS_SERVER=${flag})"
      return 0
      ;;
    1|true|yes) ;;
    auto|*)
      case "${distro}" in
        k3s|kind|minikube|microk8s) ;;
        *)
          echo "Skipping metrics-server on ${distro} (set AETHER_INSTALL_METRICS_SERVER=1 to force)"
          return 0
          ;;
      esac
      ;;
  esac

  if aether__kubectl_exec "${kubectl_bin}" top nodes --no-headers &>/dev/null; then
    echo "metrics-server already available"
    return 0
  fi

  echo "Installing metrics-server..."
  aether__kubectl_exec "${kubectl_bin}" apply -f \
    https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml

  aether__kubectl_exec "${kubectl_bin}" patch deployment metrics-server -n kube-system --type=json \
    -p='[{"op":"add","path":"/spec/template/spec/containers/0/args/-","value":"--kubelet-insecure-tls"}]' \
    2>/dev/null || true

  local i
  for i in $(seq 1 60); do
    if aether__kubectl_exec "${kubectl_bin}" top nodes --no-headers &>/dev/null; then
      echo "metrics-server ready"
      return 0
    fi
    sleep 2
  done
  echo "WARN: metrics-server installed but kubectl top still failing" >&2
  return 0
}

# Record Cilium connectivity probe result for the API (ConfigMap in aether-system).
aether_write_cilium_connectivity_status() {
  local ns="${1:?namespace}"
  local kubectl_bin="${2:-kubectl}"
  local status="${3:?status}"
  aether__kubectl_exec "${kubectl_bin}" create configmap aether-cilium-connectivity \
    -n "${ns}" \
    --from-literal=status="${status}" \
    --from-literal=checked_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    --dry-run=client -o yaml | aether__kubectl_exec "${kubectl_bin}" apply -f -
}

# Probe Cilium agent health after bootstrap (lightweight; not full connectivity test).
aether_probe_cilium_connectivity() {
  local ns="${1:?namespace}"
  local kubectl_bin="${2:-kubectl}"
  local skip="${AETHER_SKIP_CILIUM_CONNECTIVITY:-}"

  if [ "${skip}" = "1" ] || [ "${skip}" = "true" ]; then
    echo "Skipping Cilium connectivity probe (AETHER_SKIP_CILIUM_CONNECTIVITY)"
    aether_write_cilium_connectivity_status "${ns}" "${kubectl_bin}" "skipped"
    return 0
  fi

  if ! aether__kubectl_exec "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    aether_write_cilium_connectivity_status "${ns}" "${kubectl_bin}" "skipped"
    return 0
  fi

  if ! aether__kubectl_exec "${kubectl_bin}" get ds cilium -n kube-system &>/dev/null; then
    echo "WARN: Cilium CRDs present but cilium daemonset not found" >&2
    aether_write_cilium_connectivity_status "${ns}" "${kubectl_bin}" "failed"
    return 1
  fi

  local ready
  ready="$(aether__kubectl_exec "${kubectl_bin}" get ds cilium -n kube-system \
    -o jsonpath='{.status.numberReady}' 2>/dev/null || echo 0)"
  if [ "${ready:-0}" -gt 0 ] 2>/dev/null; then
    echo "Cilium agent ready (${ready} replicas)"
    aether_write_cilium_connectivity_status "${ns}" "${kubectl_bin}" "ok"
    return 0
  fi

  echo "WARN: Cilium daemonset has no ready replicas" >&2
  aether_write_cilium_connectivity_status "${ns}" "${kubectl_bin}" "failed"
  return 1
}

# Optional periodic Cilium probe Job (manifest in deploy/k8s/bootstrap/).
aether_apply_cilium_connectivity_cronjob() {
  local root="${1:?repo root}"
  local ns="${2:?namespace}"
  local kubectl_bin="${3:-kubectl}"
  local manifest="${root}/deploy/k8s/bootstrap/cilium-connectivity-cronjob.yaml"

  if [ "${AETHER_SKIP_CILIUM_CONNECTIVITY:-}" = "1" ] || [ "${AETHER_SKIP_CILIUM_CONNECTIVITY:-}" = "true" ]; then
    return 0
  fi
  if ! aether__kubectl_exec "${kubectl_bin}" get crd ciliumnetworkpolicies.cilium.io &>/dev/null; then
    return 0
  fi
  if [ ! -f "${manifest}" ]; then
    return 0
  fi
  sed "s/__AETHER_NAMESPACE__/${ns}/g" "${manifest}" | aether__kubectl_exec "${kubectl_bin}" apply -f -
  echo "Applied Cilium connectivity CronJob (namespace ${ns})"
}
