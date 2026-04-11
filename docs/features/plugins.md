# Runtime Plugin System 🔌

Extend Orchestr8 with custom runtimes via a JSON manifest-based plugin system.

---

## Overview

Plugins allow third-party runtimes to be registered and invoked alongside the built-in Podman, Kubernetes, KubeVirt, and Metal3 adapters. Plugins are discovered from `~/.orchestr8/plugins/` as `*.json` manifest files.

---

## Plugin Manifest Format

```json
{
  "name": "wasm-runtime",
  "version": "0.1.0",
  "runtime_kind": "wasm",
  "command": "/usr/local/bin/wasm-adapter",
  "capabilities": ["build", "run", "stop", "status", "delete"]
}
```

| Field | Description |
|-------|-------------|
| `name` | Unique plugin name (registry key) |
| `version` | Semantic version |
| `runtime_kind` | Custom runtime identifier |
| `command` | Path to plugin binary |
| `capabilities` | Supported operations |

---

## Commands

```bash
# Discover plugins from ~/.orchestr8/plugins/
orchestr8 plugin discover

# List registered plugins
orchestr8 plugin list

# Register from manifest file
orchestr8 plugin register /path/to/manifest.json

# Remove a plugin
orchestr8 plugin remove wasm-runtime
```

---

## Plugin Protocol

Plugins communicate via JSON-RPC style messages (stdin/stdout):

| Request | Response | Description |
|---------|----------|-------------|
| `BuildRequest` | `BuildResponse` | Build an image from spec |
| `RunRequest` | `RunResponse` | Run a workload |
| `StopRequest` | `StopResponse` | Stop an instance |
| `StatusRequest` | `StatusResponse` | Get instance status |
| `DeleteRequest` | `DeleteResponse` | Delete an instance |
| `ListRequest` | `ListResponse` | List all instances |

---

## REST API

```bash
# List plugins
curl http://localhost:8080/api/plugins

# Discover plugins
curl -X POST http://localhost:8080/api/plugins/discover
```

---

## Creating a Plugin

1. Create a binary that reads JSON from stdin and writes JSON to stdout
2. Write a manifest JSON file with name, version, runtime_kind, command, capabilities
3. Place the manifest in `~/.orchestr8/plugins/`
4. Run `orchestr8 plugin discover`
