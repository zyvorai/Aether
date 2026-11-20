---
hero:
  eyebrow: EXAMPLES & LABS
  title: OPA bundle sync with Helm
  tone: teal
---

When `opa.bundle.enabled` is true, the chart renders a ConfigMap `*-opa-bundle` with starter Rego.

## Wire OPA

1. Deploy [OPA](https://www.openpolicyagent.org/docs/latest/kubernetes/) in the cluster.
2. Mount the Aether bundle ConfigMap into OPA’s `/policies` path (or use OPA’s bundle loader).
3. Set Helm values:

```yaml
opa:
  enabled: true
  url: http://opa.opa.svc:8181
  enforce: false
  package: aether.k8s.admit
  bundle:
    enabled: true
```

4. Reload OPA after ConfigMap changes (sidecar, `kubectl rollout restart`, or Stakater Reloader).

## Aether API

- `POST /api/policy/opa` — evaluate YAML/manifest
- Cluster apply enforces when `AETHER_OPA_ENFORCE=true`
