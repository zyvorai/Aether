---
hero:
  eyebrow: REFERENCE
  title: Workload Schema Documentation
  tone: rust
---

This document describes how to use the Aether workload JSON Schema for IDE autocomplete and validation.

## Overview

The Aether workload schema (`schema/workload.schema.json`) provides:
- **Autocomplete**: IDE suggestions for fields and values
- **Validation**: Real-time error checking in your editor
- **Documentation**: Inline field descriptions

## IDE Setup

### Visual Studio Code

The repository includes VS Code settings in `.vscode/settings.json` that automatically associate the schema with workload YAML files.

**Manual Setup:**

1. Install the [YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
2. The schema is automatically applied to:
   - `workload*.yaml`
   - `examples/*.yaml`
   - Any file named `workload.yaml`

**Custom File Associations:**

Add to your workspace or user `settings.json`:

```json
{
  "yaml.schemas": {
    "./schema/workload.schema.json": ["my-custom-workload.yaml"]
  }
}
```

### JetBrains IDEs (IntelliJ, PyCharm, etc.)

1. Open **Settings** → **Languages & Frameworks** → **Schemas and DTDs** → **JSON Schema Mappings**
2. Click **+** to add a new schema
3. Set:
   - **Name**: Aether Workload
   - **Schema file or URL**: `<project-path>/schema/workload.schema.json`
   - **Schema version**: JSON Schema version 7
4. Add file path pattern: `workload*.yaml` or specific files

### Neovim/Vim with LSP

Using [yaml-language-server](https://github.com/redhat-developer/yaml-language-server):

Add to your LSP configuration:

```lua
require('lspconfig').yamlls.setup {
  settings = {
    yaml = {
      schemas = {
        ["./schema/workload.schema.json"] = "workload*.yaml"
      }
    }
  }
}
```

### Online Editors

For online YAML editors that support JSON Schema, reference the schema URL:

```yaml
# yaml-language-server: $schema=./schema/workload.schema.json

apiVersion: aether/v1
kind: Workload
...
```

## Volume Mounting

ConfigMaps and Secrets can be mounted as volumes in Kubernetes deployments by specifying a `mount_path` field:

```yaml
config:
  configMaps:
    - name: app-config
      mount_path: /etc/app           # Optional: mount as read-only volume
      data:
        app.conf: "key=value"
  secrets:
    - name: tls-certs
      mount_path: /etc/tls           # Optional: mount as read-only volume
      data:
        tls.crt: "<cert>"
```

When `mount_path` is specified, the ConfigMap or Secret is mounted as a read-only volume at that path inside the container. Without `mount_path`, values are injected as environment variables.

PVCs are auto-mounted at `/data` when `persistence.enabled: true`.

---

## NetworkPolicyConfig

The `network.network_policy` field allows you to declare Kubernetes NetworkPolicy rules directly in the workload spec. When deploying to Kubernetes, Aether generates a `NetworkPolicy` resource alongside the workload.

```yaml
network:
  service: true
  ports:
    - name: http
      port: 8080
      protocol: TCP
  network_policy:
    deny_all_ingress: true
    deny_all_egress: false
    allow_from:
      - app: frontend
      - role: monitoring
    allow_to:
      - app: database
```

### NetworkPolicyConfig Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `deny_all_ingress` | boolean | No | When `true`, deny all inbound traffic by default |
| `deny_all_egress` | boolean | No | When `true`, deny all outbound traffic by default |
| `allow_from` | array of label maps | No | Pod label selectors allowed to send traffic to this workload |
| `allow_to` | array of label maps | No | Pod label selectors this workload is allowed to send traffic to |

When `deny_all_ingress` is `true` and `allow_from` entries are provided, the generated NetworkPolicy creates ingress rules that permit traffic only from pods matching the specified labels. The same logic applies to `deny_all_egress` and `allow_to`.

---

## ScalingMetric — Custom Metrics

The `scaling.metrics` array supports a `Custom` metric type for HPA scaling on application-specific signals. When `metricType` is `Custom`, the optional `metric_name` field specifies the Pods metric name used in the HPA `PodsMetricSource`.

```yaml
scaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - metricType: CPU
      targetValue: "80%"
    - metricType: Custom
      metric_name: "requests_per_second"
      targetValue: "1000"
    - metricType: Custom
      metric_name: "queue_depth"
      targetValue: "50"
```

### ScalingMetric Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `metricType` | string (enum) | Yes | `CPU`, `Memory`, or `Custom` |
| `targetValue` | string | Yes | Target value (percentage for CPU/Memory, absolute for Custom) |
| `metric_name` | string | No | Metric name for `Custom` type (used as `PodsMetricSource.metric.name` in HPA) |

---

## IntentSpec

The `intent` field allows you to declare high-level operational goals for a workload. The intent engine uses these goals to influence runtime selection scoring, placement decisions, and policy evaluation.

```yaml
intent:
  goal: low-latency | high-throughput | cost-optimized | balanced
  sla:
    maxLatencyMs: 50
    minAvailabilityPct: 99.9
  budget:
    maxMonthlyUsd: 500
  resilience: best-effort | standard | high
  compliance:
    isolationRequired: true
    encryptionRequired: false
```

### IntentSpec Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `goal` | string (enum) | Yes | Optimization goal: `low-latency`, `high-throughput`, `cost-optimized`, or `balanced` |
| `sla.maxLatencyMs` | integer | No | Maximum acceptable latency in milliseconds |
| `sla.minAvailabilityPct` | float | No | Minimum availability percentage (e.g., `99.9`) |
| `budget.maxMonthlyUsd` | integer | No | Maximum monthly cost in USD |
| `resilience` | string (enum) | No | Resilience level: `best-effort`, `standard`, or `high` |
| `compliance.isolationRequired` | boolean | No | Whether workload isolation is required |
| `compliance.encryptionRequired` | boolean | No | Whether encryption at rest is required |

### Example: Low-Latency Intent

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: trading-api
  owner: platform
  project: fintech

intent:
  goal: low-latency
  sla:
    maxLatencyMs: 10
    minAvailabilityPct: 99.99
  budget:
    maxMonthlyUsd: 2000
  resilience: high
  compliance:
    isolationRequired: true
    encryptionRequired: true

requirements:
  cpu: "4"
  memory: 8Gi
  storage: 50Gi
```

Use `aether intent` to evaluate and display the intent scoring for a workload spec.

---

## Schema Features

### Field Validation

The schema validates:
- **Required fields**: Ensures all mandatory fields are present
- **Type checking**: Validates data types (string, integer, boolean, etc.)
- **Enums**: Restricts values to allowed options
- **Patterns**: DNS names, ports, resource quantities

### Examples

**CPU/Memory Validation:**
```yaml
requirements:
  cpu: "2"        # Valid: "2", "2000m", "0.5"
  memory: "4Gi"   # Valid: "4Gi", "2048Mi"
  storage: "20Gi" # Valid: "20Gi", "100Mi"
```

**Runtime Validation:**
```yaml
runtime:
  preferred: auto  # Enum: auto, container, kube, kubevirt, metal
  allow:
    - container    # Must have at least one
    - kube
```

**Port Validation:**
```yaml
network:
  ports:
    - containerPort: 8080  # 1-65535
      servicePort: 80      # 1-65535
      protocol: TCP        # Enum: TCP, UDP
```

### Autocomplete Examples

When you type in a workload YAML file, the IDE will suggest:

1. **Top-level fields**:
   ```yaml
   apiVersion: aether/v1
   kind: Workload
   metadata:  # ← Autocomplete suggests: metadata, build, requirements, runtime
   ```

2. **Enum values**:
   ```yaml
   runtime:
     preferred: # ← Autocomplete suggests: auto, container, kube, kubevirt, metal
   ```

3. **Nested structures**:
   ```yaml
   health:
     liveness:  # ← Autocomplete suggests: httpGet, tcpSocket, exec
       httpGet:
         path:    # ← Autocomplete suggests field with description
         port:
   ```

## Schema Validation Examples

### Valid Workload

```yaml
apiVersion: aether/v1
kind: Workload

metadata:
  name: my-app
  owner: devteam
  project: production

build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/myorg

requirements:
  cpu: "2"
  memory: 4Gi
  storage: 20Gi

runtime:
  preferred: auto
  allow:
    - container
    - kube
```

### Invalid Examples

**Missing Required Field:**
```yaml
apiVersion: aether/v1
kind: Workload
metadata:
  name: my-app
  # ERROR: Missing 'owner' and 'project' (required fields)
```

**Invalid Enum Value:**
```yaml
runtime:
  preferred: docker  # ERROR: Must be one of: auto, container, kube, kubevirt, metal
```

**Invalid Port Range:**
```yaml
network:
  ports:
    - containerPort: 99999  # ERROR: Must be 1-65535
      servicePort: 80
```

**Type Mismatch:**
```yaml
scaling:
  minReplicas: "two"  # ERROR: Must be integer, not string
```

## Using the Schema in CI/CD

Validate workload files in CI using tools like `ajv-cli`:

```bash
# Install ajv-cli
npm install -g ajv-cli

# Validate workload file
ajv validate -s schema/workload.schema.json -d workload.yaml
```

Or use the built-in Aether validation:

```bash
aether -s workload.yaml validate
```

## Schema Updates

The schema is maintained alongside the Rust code. When updating the `Workload` struct in `src/spec.rs`, remember to update `schema/workload.schema.json` accordingly.

### Contribution Guidelines

When adding new fields:

1. Update `src/spec.rs` with the new field
2. Update `schema/workload.schema.json` with:
   - Field definition
   - Type information
   - Description
   - Validation rules (if applicable)
3. Add examples to this documentation
4. Test with IDE autocomplete

## Resources

- [JSON Schema Specification](https://json-schema.org/)
- [YAML Language Server](https://github.com/redhat-developer/yaml-language-server)
- [VS Code YAML Extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
- [Aether Documentation](https://github.com/zyvorai/Aether/blob/main/docs/README.md)
