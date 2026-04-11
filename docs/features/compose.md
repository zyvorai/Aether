# Compose: Multi-Workload Deployment 📦

Deploy multiple workloads as a single unit with dependency ordering and environment injection.

---

## Compose File Format

Create `orchestr8-compose.yaml`:

```yaml
version: "1"
workloads:
  database:
    spec: db.yaml
    runtime: container
    env:
      POSTGRES_DB: myapp
      POSTGRES_USER: admin

  api:
    spec: api.yaml
    depends_on: [database]
    env:
      DATABASE_URL: postgres://admin@database:5432/myapp
      LOG_LEVEL: debug

  web:
    spec: web.yaml
    depends_on: [api]
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `version` | Yes | Schema version (currently `"1"`) |
| `workloads` | Yes | Named workload entries |
| `workloads.<name>.spec` | Yes | Path to workload YAML spec |
| `workloads.<name>.runtime` | No | Runtime override (e.g., `container`, `kube`) |
| `workloads.<name>.depends_on` | No | Workloads that must start before this one |
| `workloads.<name>.env` | No | Environment variables injected at deploy time |

---

## Commands

```bash
# Validate compose file (checks deps, circular references)
orchestr8 compose validate

# Deploy all in dependency order
orchestr8 compose up

# Deploy with runtime override
orchestr8 compose up --runtime kube

# Preview without executing
orchestr8 compose up --dry-run

# Stop all in reverse dependency order
orchestr8 compose down
```

---

## Dependency Resolution

Dependencies are resolved using topological sort (Kahn's algorithm):

```
database (no deps) → api (depends on database) → web (depends on api)
```

Circular dependencies are detected and rejected:

```bash
$ orchestr8 compose validate
# Error: circular dependency detected in compose workloads
```

---

## Environment Injection

The `env` field injects environment variables into workloads:

- **Podman:** Added as `-e KEY=VALUE` flags
- **Kubernetes:** Added as inline `env:` entries on containers

Environment variables from compose override any existing config in the workload spec.

---

## REST API

```bash
# Validate via API
curl -X POST http://localhost:8080/api/compose/validate \
  -H "Content-Type: text/plain" \
  -d @orchestr8-compose.yaml
```
