# Example OPA policy for Aether cluster apply admission.
# Load: opa run --server examples/opa/aether-k8s-admit.rego
# Set AETHER_OPA_URL=http://127.0.0.1:8181 and AETHER_OPA_PACKAGE=aether.k8s.admit

package aether.k8s.admit

default allow := false

allow if {
	count(deny) == 0
}

deny[msg] if {
	not input.manifest.metadata.labels.owner
	msg := "missing label owner on metadata.labels"
}

deny[msg] if {
	input.manifest.kind == "Deployment"
	not input.manifest.spec.template.spec.containers[_].resources.limits
	msg := "Deployment containers must define resource limits"
}
