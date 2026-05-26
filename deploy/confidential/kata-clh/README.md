# Kata + Cloud Hypervisor confidential runtime classes
#
# Prerequisites:
# - confidential-containers-operator installed
# - Cloud Hypervisor configured as Kata VMM (see CC operator docs)
# - Nodes labeled for TEE (node.kubernetes.io/sev-snp or feature.node.kubernetes.io/tdx)
#
# Apply:
#   kubectl apply -f deploy/confidential/kata-clh/runtime-classes.yaml
#
# Prefer kata-clh-* over legacy kata-qemu-* for lower overhead microVMs.
# Set AETHER_KATA_HYPERVISOR=qemu to use legacy QEMU handlers.

apiVersion: node.k8s.io/v1
kind: RuntimeClass
metadata:
  name: kata-clh-snp
handler: kata-clh-snp
