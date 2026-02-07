# Workload Schema Documentation

This document describes how to use the Orchestr8 workload JSON Schema for IDE autocomplete and validation.

## Overview

The Orchestr8 workload schema (`schema/workload.schema.json`) provides:
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
   - **Name**: Orchestr8 Workload
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

apiVersion: orchestr8/v1
kind: Workload
...
```

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
   apiVersion: orchestr8/v1
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
apiVersion: orchestr8/v1
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
apiVersion: orchestr8/v1
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

Or use the built-in Orchestr8 validation:

```bash
orchestr8 -s workload.yaml validate
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
- [Orchestr8 Documentation](../README.md)
