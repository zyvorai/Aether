# 🔌 Plugin System: Custom Runtime Extensions

> Extend aether with third-party runtimes via a JSON manifest and JSON-RPC protocol.

---

## 📑 Table of Contents

- [Architecture Overview](#-architecture-overview)
- [Plugin Manifest Format](#-plugin-manifest-format)
- [Plugin Discovery](#-plugin-discovery)
- [Register / Unregister Commands](#-register--unregister-commands)
- [Plugin Protocol (JSON-RPC Style)](#-plugin-protocol-json-rpc-style)
- [Creating a Custom Runtime Plugin](#-creating-a-custom-runtime-plugin)
- [REST API Endpoints](#-rest-api-endpoints)
- [Persistent Registry](#-persistent-registry)
- [Cross-References](#-cross-references)

---

## 🏗️ Architecture Overview

The aether plugin system enables any external binary to act as a runtime adapter. The architecture follows a straightforward pattern:

```
┌──────────────┐     JSON-RPC      ┌──────────────────┐
│  aether   │  ─────────────►   │  Plugin Binary   │
│  (CLI/API)   │  ◄─────────────   │  (any language)  │
└──────────────┘    stdin/stdout    └──────────────────┘
        │
        ▼
┌──────────────────────────────┐
│  ~/.aether/plugins.json   │   Persistent registry
└──────────────────────────────┘
```

### Design Summary

| Aspect | Decision |
|---|---|
| Discovery | Scan `~/.aether/plugins/` for `*.json` manifest files |
| Communication | JSON-RPC style messages over stdin/stdout |
| Registry | Persistent `plugins.json` keyed by plugin name |
| Capabilities | Each plugin declares which operations it supports |
| Language | Plugin binaries can be written in **any language** |
| Versioning | Semantic versioning in manifests; re-register to upgrade |

---

## 📋 Plugin Manifest Format

A plugin manifest is a JSON file that describes the plugin binary and its capabilities.

```json
{
  "name": "my-runtime",
  "version": "0.1.0",
  "runtime_kind": "custom-wasm",
  "command": "/usr/local/bin/my-runtime",
  "capabilities": ["build", "run", "stop", "status"]
}
```

### Field Reference

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Human-readable plugin name. Used as the registry key -- must be unique. |
| `version` | string | Yes | Semantic version of the plugin (e.g. `"1.2.3"`). |
| `runtime_kind` | string | Yes | Custom runtime identifier this plugin provides. Used in compose files and CLI `--runtime` flags. |
| `command` | string | Yes | Absolute path to the plugin binary. |
| `capabilities` | array | Yes | List of supported operations: `"build"`, `"run"`, `"stop"`, `"status"`, `"delete"`, `"list"`. |

### Capabilities Matrix

| Capability | Description | Required? |
|---|---|---|
| `build` | Build an image from a workload spec | Recommended |
| `run` | Run a workload instance | **Required** |
| `stop` | Stop a running instance | **Required** |
| `status` | Query instance status | Recommended |
| `delete` | Delete an instance permanently | Optional |
| `list` | List all managed instances | Optional |

### Example Manifests

**WASM Runtime:**

```json
{
  "name": "wasm-plugin",
  "version": "1.2.3",
  "runtime_kind": "wasm",
  "command": "/opt/wasm-runner",
  "capabilities": ["run", "stop"]
}
```

**Firecracker MicroVM:**

```json
{
  "name": "firecracker",
  "version": "2.0.0",
  "runtime_kind": "microvm",
  "command": "/usr/local/bin/fc-adapter",
  "capabilities": ["build", "run", "stop", "status", "delete", "list"]
}
```

---

## 🔍 Plugin Discovery

aether discovers plugins by scanning `~/.aether/plugins/` for `*.json` manifest files.

### Directory Structure

```
~/.aether/
├── plugins/
│   ├── wasm-runtime.json        # ✅ manifest for WASM plugin
│   ├── firecracker.json         # ✅ manifest for Firecracker plugin
│   ├── readme.txt               # ❌ ignored (not .json)
│   └── bad-syntax.json          # ⚠️ skipped with warning (invalid JSON)
└── plugins.json                 # 📁 persistent registry (auto-managed)
```

### Discovery Rules

| Condition | Behavior |
|---|---|
| `.json` extension | File is loaded and parsed |
| Non-`.json` file | Silently skipped |
| Invalid JSON | Skipped with a warning log |
| Valid manifest | Merged into registry (keyed by `name`) |
| Directory missing | Returns 0 discovered (no error) |
| Duplicate names | Later discovery overwrites earlier entry |

### Running Discovery

```bash
# Discover plugins from the default directory
aether plugin discover
# ✅ Discovered 2 plugins (3 total registered)
```

---

## ⚡ Register / Unregister Commands

### List Registered Plugins

```bash
aether plugin list
```

Displays a table of all registered plugins:

```
╭──────────────────┬─────────┬──────────────┬─────────────────────────────╮
│ Name             │ Version │ Runtime Kind │ Command                     │
├──────────────────┼─────────┼──────────────┼─────────────────────────────┤
│ wasm-runtime     │ 1.0.0   │ wasm         │ /usr/local/bin/wasm-runner  │
│ firecracker      │ 0.3.0   │ microvm      │ /opt/fc/fc-runtime          │
╰──────────────────┴─────────┴──────────────┴─────────────────────────────╯
```

---

### Register a Plugin from a Manifest File

```bash
aether plugin register ./my-plugin.json
```

This reads the manifest file and adds it to the persistent registry. If a plugin with the same `name` already exists, it is **overwritten** with the new manifest (useful for upgrades).

---

### Unregister a Plugin

```bash
aether plugin remove my-runtime
```

Removes the plugin from the registry by name. Returns the removed manifest if present, or a warning if the plugin was not found.

---

### Discover Plugins

```bash
aether plugin discover
```

Scans `~/.aether/plugins/` and merges all valid manifests into the registry.

---

## 📡 Plugin Protocol (JSON-RPC Style)

Plugins communicate with aether via a tagged JSON protocol over stdin/stdout. Each message includes a `type` field that identifies the request or response kind.

### Request/Response Summary

| Request | Response | Fields |
|---|---|---|
| `BuildRequest` | `BuildResponse` | `spec_json` → `image_json` |
| `RunRequest` | `RunResponse` | `image_json` + `spec_json` → `instance_json` |
| `StopRequest` | `StopResponse` | `instance_json` → `success` (bool) |
| `StatusRequest` | `StatusResponse` | `instance_json` → `status_json` |
| `DeleteRequest` | `DeleteResponse` | `instance_json` → `success` (bool) |
| `ListRequest` | `ListResponse` | (none) → `instances_json` |

### Request Message Examples

#### `BuildRequest`

```json
{
  "type": "BuildRequest",
  "spec_json": "{\"name\": \"my-app\", \"image\": \"my-app:v1\"}"
}
```

#### `RunRequest`

```json
{
  "type": "RunRequest",
  "image_json": "{\"tag\": \"v1\"}",
  "spec_json": "{\"cpu\": \"2\", \"memory\": \"4Gi\"}"
}
```

#### `StopRequest`

```json
{
  "type": "StopRequest",
  "instance_json": "{\"id\": \"inst-abc123\"}"
}
```

#### `StatusRequest`

```json
{
  "type": "StatusRequest",
  "instance_json": "{\"id\": \"inst-abc123\"}"
}
```

#### `DeleteRequest`

```json
{
  "type": "DeleteRequest",
  "instance_json": "{\"id\": \"inst-abc123\"}"
}
```

#### `ListRequest`

```json
{
  "type": "ListRequest"
}
```

### Response Message Examples

#### `BuildResponse`

```json
{
  "type": "BuildResponse",
  "image_json": "{\"name\": \"custom-img\", \"tag\": \"latest\"}"
}
```

#### `RunResponse`

```json
{
  "type": "RunResponse",
  "instance_json": "{\"id\": \"inst-abc123\", \"name\": \"my-app\"}"
}
```

#### `StopResponse` / `DeleteResponse`

```json
{
  "type": "StopResponse",
  "success": true
}
```

#### `StatusResponse`

```json
{
  "type": "StatusResponse",
  "status_json": "{\"state\": \"running\", \"ready\": true}"
}
```

#### `ListResponse`

```json
{
  "type": "ListResponse",
  "instances_json": "[{\"id\": \"inst-001\"}, {\"id\": \"inst-002\"}]"
}
```

### Protocol Flow

```
aether ──stdin──►  {"type":"BuildRequest","spec_json":"{...}"}
plugin    ──stdout──► {"type":"BuildResponse","image_json":"{...}"}

aether ──stdin──►  {"type":"RunRequest","image_json":"{...}","spec_json":"{...}"}
plugin    ──stdout──► {"type":"RunResponse","instance_json":"{...}"}
```

### Serialization Notes

- The `type` field is a **tagged enum** (serde `#[serde(tag = "type")]`)
- Payload fields (`spec_json`, `instance_json`, etc.) are **JSON strings** -- the plugin is responsible for parsing them
- One message per line (newline-delimited JSON)

---

## 🏗️ Plugin Runtime Implementation

When a plugin is registered, Aether can use it as a full `Runtime` implementation via the `PluginRuntime` struct. This means plugins participate in the same lifecycle as built-in runtimes (Podman, Kubernetes, KubeVirt, Metal3).

### How It Works

1. **Binary validation:** On creation, `PluginRuntime` verifies the plugin binary exists at the path specified in the manifest
2. **Capability checking:** Before each operation, the plugin's `capabilities` list is checked -- calling `build` on a plugin that only supports `["run", "stop"]` returns an error
3. **IPC call:** A JSON-RPC message is written to the plugin's stdin, and the response is read from stdout
4. **Timeout:** Each IPC call has a **60-second timeout** -- plugins that hang are terminated with an error
5. **Error handling:** Non-zero exit codes, invalid JSON responses, and unexpected message types all produce descriptive errors

### Operation Flow

```
aether CLI
     │
     ▼
PluginRuntime::build(spec)
     │  1. Check "build" in capabilities
     │  2. Serialize spec to JSON
     │  3. Spawn plugin binary
     │  4. Write {"type":"BuildRequest","spec_json":"..."} to stdin
     │  5. Read stdout (60s timeout)
     │  6. Parse {"type":"BuildResponse","image_json":"..."}
     │  7. Deserialize image from JSON
     ▼
Image returned to caller
```

### Error Scenarios

| Scenario | Error Message |
|---|---|
| Binary not found | `Plugin binary '/path/to/bin' not found for plugin 'name'` |
| Missing capability | `Plugin 'name' does not support 'build' (capabilities: run, stop)` |
| Timeout after 60s | `Plugin 'name' timed out after 60s` |
| Non-zero exit | `Plugin 'name' exited with exit status 1: <stderr>` |
| Invalid JSON response | `Plugin 'name' returned invalid JSON: <parse error> (raw: ...)` |
| Unexpected response type | `Unexpected response from plugin: <type>` |

### Plugin Discovery in Runtime Factory

The `create_plugin_runtime()` function searches the plugin registry for a plugin whose `runtime_kind` matches the requested runtime name. This enables custom runtimes to be used anywhere a built-in runtime is accepted:

```bash
# Use a plugin-provided runtime in compose files
# runtime: wasm  ←  matches a plugin with runtime_kind: "wasm"
aether compose up
```

### Unsupported Operations

- **Logs:** Returns `"Plugin log streaming not yet supported"` (plugins do not yet implement log forwarding)
- **List without capability:** Returns an empty list instead of an error

---

## 🛠️ Creating a Custom Runtime Plugin

This walkthrough creates a minimal plugin in Bash. Plugins can be written in **any language** (Rust, Go, Python, Node.js, etc.).

### Step 1: Create the Plugin Binary

```bash
#!/usr/bin/env bash
# File: /usr/local/bin/my-custom-runtime
set -euo pipefail

while IFS= read -r line; do
  type=$(echo "$line" | jq -r '.type')

  case "$type" in
    BuildRequest)
      echo '{"type":"BuildResponse","image_json":"{\"name\":\"custom-img\",\"tag\":\"latest\"}"}'
      ;;
    RunRequest)
      ID="inst-$(date +%s)"
      echo "{\"type\":\"RunResponse\",\"instance_json\":\"{\\\"id\\\":\\\"$ID\\\"}\"}"
      ;;
    StopRequest)
      echo '{"type":"StopResponse","success":true}'
      ;;
    StatusRequest)
      echo '{"type":"StatusResponse","status_json":"{\"state\":\"running\",\"ready\":true}"}'
      ;;
    DeleteRequest)
      echo '{"type":"DeleteResponse","success":true}'
      ;;
    ListRequest)
      echo '{"type":"ListResponse","instances_json":"[]"}'
      ;;
  esac
done
```

```bash
chmod +x /usr/local/bin/my-custom-runtime
```

### Step 2: Create the Manifest

Save to `~/.aether/plugins/my-custom-runtime.json`:

```json
{
  "name": "my-custom-runtime",
  "version": "0.1.0",
  "runtime_kind": "custom",
  "command": "/usr/local/bin/my-custom-runtime",
  "capabilities": ["build", "run", "stop", "status", "delete", "list"]
}
```

### Step 3: Discover and Verify

```bash
# Create the plugins directory if needed
mkdir -p ~/.aether/plugins

# Discover the plugin
aether plugin discover
# ✅ Discovered 1 plugin

# Verify it's registered
aether plugin list
# ╭─────────────────────┬─────────┬──────────────┬───────────────────────────────────╮
# │ Name                │ Version │ Runtime Kind │ Command                           │
# ├─────────────────────┼─────────┼──────────────┼───────────────────────────────────┤
# │ my-custom-runtime   │ 0.1.0   │ custom       │ /usr/local/bin/my-custom-runtime  │
# ╰─────────────────────┴─────────┴──────────────┴───────────────────────────────────╯
```

### Step 4: Use in a Compose File

```yaml
version: "1"
workloads:
  my-app:
    spec: ./app.yaml
    runtime: custom    # matches runtime_kind from the manifest
```

### Step 5: Upgrade the Plugin

To upgrade, update the manifest file and re-discover:

```bash
# Update version in the manifest
# Then:
aether plugin discover
# or:
aether plugin register ~/.aether/plugins/my-custom-runtime.json
```

---

## 🌐 REST API Endpoints

### GET `/api/plugins` -- List Registered Plugins

```bash
curl http://localhost:8080/api/plugins
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "name": "wasm-runtime",
      "version": "1.0.0",
      "runtime_kind": "wasm",
      "command": "/usr/local/bin/wasm-runner",
      "capabilities": ["run", "stop"]
    }
  ],
  "error": null
}
```

---

### POST `/api/plugins/discover` -- Trigger Plugin Discovery

```bash
curl -X POST http://localhost:8080/api/plugins/discover
```

**Response:**

```json
{
  "success": true,
  "data": {
    "discovered": 2,
    "total": 3
  },
  "error": null
}
```

---

## 💾 Persistent Registry

The plugin registry is stored at `~/.aether/plugins.json`. It persists across CLI invocations and API server restarts.

### Registry Structure

```json
{
  "plugins": {
    "wasm-runtime": {
      "name": "wasm-runtime",
      "version": "1.0.0",
      "runtime_kind": "wasm",
      "command": "/usr/local/bin/wasm-runner",
      "capabilities": ["run", "stop"]
    },
    "firecracker": {
      "name": "firecracker",
      "version": "2.0.0",
      "runtime_kind": "microvm",
      "command": "/usr/local/bin/fc-adapter",
      "capabilities": ["build", "run", "stop", "status", "delete", "list"]
    }
  }
}
```

### Registry Behaviors

| Operation | Effect |
|---|---|
| `discover` | Scans directory, merges manifests, saves registry |
| `register` | Reads manifest file, inserts/updates entry, saves |
| `remove` | Deletes entry by name, saves |
| Load from missing file | Returns empty registry (no error) |
| Duplicate name on register | Overwrites existing entry |

---

## 🔗 Cross-References

| Document | Relevance |
|---|---|
| [Compose Guide](./compose.md) | Use plugins as runtime targets in compose files |
| [Security Guide](./security.md) | Policy enforcement for plugin-deployed workloads |
| [API Reference](../reference/api/API-Reference.md) | Full REST API documentation |
| [Quick Reference](../quick-reference/QUICK_REFERENCE.md) | Command cheat sheet |
