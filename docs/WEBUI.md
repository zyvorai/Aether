# WebUI and REST API Guide

Aether provides a full-featured React web dashboard and REST API for managing workloads through your browser.

## Overview

The WebUI is a **React 18 single-page application** built with TypeScript, Tailwind CSS, and Vite. It provides:
- **AI Infrastructure OS navigation** (Pro view): 12 sections — Overview (Command Center), Fleet, Fabric, Workloads, AI Studio, Migrations, Observability, Security, Cost, GitOps, Labs, Settings
- **37+ dashboard views** including AI/copilot/intelligence, confidential computing, fleet/clusters, GitOps, and legacy deep links (see `web/dashboard/src/utils/dashboardNav.ts`)
- **Command Center** (`GET /api/command-center/briefing`) — narrative fleet briefing on `/`
- **Runtime Fabric** (`/fabric`) — live Application → Runtime → Cluster topology graph
- **Copilot rail** — permanent Ask Aether sidebar (xl+); full-page copilot at `/copilot`
- **REST API**: Programmatic access to all operations (43+ endpoints)
- **Dark metallic zinc theme**: Consistent dark UI with orange accents
- **Responsive design**: Desktop and mobile support with collapsible menu
- **Real-time updates**: Auto-refresh on key pages

### Tech Stack

| Component | Technology |
|---|---|
| **Framework** | React 18 |
| **Language** | TypeScript |
| **Styling** | Tailwind CSS |
| **Build tool** | Vite |
| **Location** | `web/dashboard/` |

## Quick Start

### Start the API Server

```bash
aether serve
```

The server starts on `http://127.0.0.1:5090` by default and serves the built dashboard assets.

### Custom Host and Port

```bash
aether serve --host 0.0.0.0 --port 3000
```

### Development Mode

For frontend development with hot-reload:

```bash
cd web/dashboard
npm install
npm run dev
```

The Vite dev server starts with a proxy to `http://localhost:5090` for API requests.

### Production Build

```bash
cd web/dashboard && npm run build
```

The build output is served by the Aether API server at `/`.

### Enable API Authentication

Set the `AETHER_API_KEY` environment variable to require Bearer token authentication on all `/api/*` endpoints:

```bash
export AETHER_API_KEY="my-secure-api-key"
aether serve
```

When enabled, all API requests must include:
```
Authorization: Bearer my-secure-api-key
```

The `/health` endpoint and dashboard (`/`) remain public. See [Security Features](features/security.md#-api-authentication) for details.

### Access the Dashboard

Open your browser and navigate to:
```
http://localhost:5090
```

**Default login credentials:** `admin` / `aether`

## Dashboard Pages

The dashboard provides 19 pages organized under a top navigation bar with dropdown groups:

| Page | Description |
|---|---|
| **Dashboard** | Overview with workload stats, runtime distribution, and recent events |
| **Workloads** | List, create, start, stop, delete, and view logs for workloads |
| **AI Engine** | Runtime recommendations, profiling, log analysis, and scaling advice |
| **Cost** | Cost estimation across cloud providers |
| **Affinity** | Runtime affinity scores by workload class |
| **Drift** | Configuration drift detection and reconciliation |
| **Policy** | Policy enforcement checks against production/development rule sets |
| **Scheduler** | Runtime utilization and optimization suggestions |
| **Health** | Health monitoring timeline and uptime summaries |
| **Events** | Event stream with severity filtering and summary |
| **SLA** | SLA compliance targets and status |
| **Dependencies** | Dependency graph visualization and startup ordering |
| **Environments** | Environment configuration management |
| **Secrets** | Secret metadata and rotation status (values never exposed) |
| **Backups** | Backup creation and listing |
| **Templates** | Generate workloads from built-in templates |
| **Plugins** | Plugin registry and discovery |
| **Audit** | Audit trail with integrity verification |
| **Metrics** | Prometheus metrics display |

## Dashboard Features

### Workload-scoped deep links

Many views accept `?workload=` (and Workload detail accepts `?tab=trust` for the Trust & attestation panel). Context banners on Overview, Events, Health, Alerts, Audit, GitOps, Fleet, Policy, Compose, Intelligence, Confidential, Drift, Scheduler, Cost, Copilot, Platform, Clusters, OpenAPI, RBAC, Workloads (list mode), Affinity, Deps, AI, and search-scoped pages (Secrets, Backups, Metrics, Plugins, Templates, SLA, Envs) link to related views via **WorkloadScopedCrossLinks** (Events / Alerts / Health / Trust, plus optional Drift / Audit / GitOps / Metrics). On Metrics, Plugins, Templates, SLA, Secrets, Backups, Health, OpenAPI, and Drift, `?workload=` aliases the toolbar `?q=` filter via `useWorkloadOrSearchFilter()`. The breadcrumb shows a workload segment (`breadcrumb-workload`) when `?workload=` is set. Envs pre-fills the promote form from `?workload=`. Alert rules may include a `workload` field; `GET /api/alerts/status?workload=` returns only matching rules. Copilot accepts `?workload=` plus `?q=` for the natural-language prompt. Deploy success links open Health, Events, Alerts, Drift, Trust, GitOps, Metrics, Copilot, Audit, Intelligence, Environments, and Templates. The command palette includes per-workload actions (logs, drift, events, copilot, trust, confidential, alerts, audit, health, gitops, metrics, deps, affinity, SLA, envs, templates, plugins, cost, scheduler, policy, intelligence, editor, backups, secrets, AI, drift page, scoring, start/stop) with `data-testid="command-palette-item-{id}"`. Cost, Scheduler, Affinity, Deps, AI, and Drift use the shared **WorkloadContextBanner** with optional legacy `openTestId`. Plugins, Templates, and SLA search banners link to the visual editor when filtered. Platform and Clusters cross-link to each other; deploy success adds Platform, OpenAPI, and RBAC shortcuts. Secrets and Backups use the shared search banner with editor/drift shortcuts; GitOps, Policy, Audit, Fleet, Compose, Confidential, Intelligence, Metrics, and Envs extend the governance cross-link mesh. Drift, Cost, Affinity, Overview, and placement pages extend the governance mesh with intelligence, copilot, secrets, and compose shortcuts. See `docs/phases-1331-1630.md` through `docs/phases-2081-2130.md` for the phase map.

### Navigation

The top navbar organizes pages into dropdown groups for quick access. On mobile devices, a hamburger menu provides access to all pages.

### SSE Real-Time Updates

The dashboard receives real-time updates from the API server via Server-Sent Events (SSE). When a mutation occurs (create, delete, start, stop, migrate), the API handler emits a `ServerEvent` that the dashboard's `useEventStream` hook receives instantly -- no polling required.

A **connection indicator** (green/red dot) in the navbar shows the live SSE connection status. Green means the SSE stream is connected and receiving events; red means the connection is lost.

### WorkloadDetail Panel

Click any workload name to open a tabbed detail view with four tabs:

| Tab | Content |
|---|---|
| **Overview** | Workload metadata, runtime, status, resource requirements, created/updated timestamps |
| **Logs** | Integrated LogViewer component (see below) |
| **Drift** | Configuration drift detection results for this workload |
| **Scoring** | AI scoring breakdown and Intent Debugger visualization |

The panel includes action buttons: **Start**, **Stop**, **Restart**, and **Delete** -- each with confirmation dialogs.

### LogViewer

A full-featured log viewer component with:

- **Auto-polling**: Fetches new log lines on a configurable interval
- **Follow mode**: Auto-scrolls to the bottom as new lines arrive (toggle on/off)
- **Filter input**: Search/filter log lines by text
- **Line numbers**: Optional line number display
- **Color-coded levels**: Log levels (INFO, WARN, ERROR, DEBUG) are color-coded for quick scanning
- **Copy-to-clipboard**: Copy the full log output or selected lines

### Command Palette

Press **Cmd+K** (macOS) or **Ctrl+K** (Linux/Windows) to open the Command Palette overlay. You can also press **?** to open it.

The palette provides fuzzy search across:
- **Pages**: Navigate directly to any dashboard page
- **Workloads**: Jump to a specific workload's detail view
- **Actions**: Execute common actions (refresh, deploy, migrate)

### Intent Debugger

The Intent Debugger visualizes AI scoring for a workload using a **pure SVG radar chart** with four axes:
- **Cost** -- how cost-effective the runtime is for this workload
- **Performance** -- latency and throughput scoring
- **Reliability** -- failure rate and restart history
- **Availability** -- uptime and SLA compliance

Each runtime's scores are overlaid on the chart for visual comparison.

### Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `r` | Refresh current page data |
| `?` | Open Command Palette |
| `Cmd+K` / `Ctrl+K` | Open Command Palette |

### Confirmation Dialogs

Destructive actions (delete, stop) display a confirmation dialog before executing. This prevents accidental data loss.

### Toast Notifications

Success and error feedback is shown via toast notifications that auto-dismiss after a few seconds.

### Error Boundary

React error boundaries catch rendering errors and display a fallback UI instead of a blank page.

### Auto-Refresh

Key pages (Dashboard, Workloads, Health, Events) automatically refresh data at regular intervals to show the latest state. SSE events trigger immediate updates for mutation-related pages.

## REST API

### Endpoints

#### Health Check
```http
GET /health
```

Response:
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "version": "0.1.0"
  }
}
```

#### List Workloads
```http
GET /api/workloads
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "name": "my-app",
      "runtime": "Kubernetes",
      "image": "ghcr.io/myorg/app:latest",
      "status": "running",
      "created_at": "2024-02-06T10:00:00Z"
    }
  ]
}
```

#### Create Workload
```http
POST /api/workloads
Content-Type: application/json
```

Request:
```json
{
  "spec": {
    "metadata": {
      "name": "new-app",
      "owner": "team",
      "project": "demo"
    },
    "image": {
      "registry": "ghcr.io",
      "repository": "myorg/app",
      "tag": "latest"
    },
    "requirements": {
      "cpu": "2",
      "memory": "4Gi",
      "storage": "20Gi"
    },
    "runtime": {
      "preferred": "Auto"
    }
  },
  "runtime": "kubernetes"
}
```

Response:
```json
{
  "success": true,
  "data": "Workload new-app created"
}
```

#### Get Workload Details
```http
GET /api/workloads/{name}
```

Response:
```json
{
  "success": true,
  "data": {
    "name": "my-app",
    "runtime": "Kubernetes",
    "image": "ghcr.io/myorg/app:latest",
    "status": "running",
    "created_at": "2024-02-06T10:00:00Z"
  }
}
```

#### Get Workload Logs
```http
GET /api/workloads/{name}/logs
```

Response:
```json
{
  "success": true,
  "data": "2024-02-06 10:15:30 [INFO] Application started\n2024-02-06 10:15:31 [INFO] Listening on :8080\n"
}
```

#### Stop Workload
```http
POST /api/workloads/{name}/stop
```

Response:
```json
{
  "success": true,
  "data": "Workload my-app stopped"
}
```

#### Delete Workload
```http
DELETE /api/workloads/{name}
```

Response:
```json
{
  "success": true,
  "data": "Workload my-app deleted"
}
```

#### Estimate Costs
```http
POST /api/cost
Content-Type: application/json
```

Request:
```json
{
  "metadata": {
    "name": "test-app",
    "owner": "team",
    "project": "demo"
  },
  "requirements": {
    "cpu": "2",
    "memory": "4Gi",
    "storage": "20Gi"
  }
}
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "provider": "Linode",
      "cpu_cost_monthly": 10.0,
      "memory_cost_monthly": 20.0,
      "storage_cost_monthly": 2.0,
      "total_monthly": 32.0,
      "total_hourly": 0.0438,
      "currency": "USD"
    }
  ]
}
```

#### List Backups
```http
GET /api/backups
```

Response:
```json
{
  "success": true,
  "data": [
    "/home/user/.aether/backups/backup-20240206-143052.json",
    "/home/user/.aether/backups/production-2024-02-06.json"
  ]
}
```

#### Create Backup
```http
POST /api/backups
Content-Type: application/json
```

Request:
```json
{
  "name": "my-backup",
  "description": "Pre-deployment backup"
}
```

Response:
```json
{
  "success": true,
  "data": "Backup created: /home/user/.aether/backups/my-backup.json"
}
```

#### List Plugins
```http
GET /api/plugins
```

Response:
```json
{
  "success": true,
  "data": [
    {
      "name": "wasm-runtime",
      "version": "0.1.0",
      "runtime_kind": "wasm",
      "command": "/usr/bin/wasm-adapter",
      "capabilities": ["build", "run", "stop"]
    }
  ]
}
```

#### Discover Plugins
```http
POST /api/plugins/discover
```

Response:
```json
{
  "success": true,
  "data": {
    "discovered": 2,
    "total": 3
  }
}
```

#### Health Summary
```http
GET /api/health/:workload
```

Response:
```json
{
  "success": true,
  "data": {
    "workload": "my-app",
    "total_checks": 50,
    "ready_checks": 48,
    "uptime_percent": 96.0,
    "last_restart_count": 1,
    "last_state": "running"
  }
}
```

#### Validate Compose
```http
POST /api/compose/validate
Content-Type: text/plain
```

Request body: YAML compose spec

Response:
```json
{
  "success": true,
  "data": {
    "valid": true,
    "workload_count": 3,
    "deploy_order": ["database", "api", "web"]
  }
}
```

## API Client Examples

### cURL

#### List all workloads
```bash
curl http://localhost:5090/api/workloads
```

#### Get workload logs
```bash
curl http://localhost:5090/api/workloads/my-app/logs
```

#### Stop a workload
```bash
curl -X POST http://localhost:5090/api/workloads/my-app/stop
```

#### Delete a workload
```bash
curl -X DELETE http://localhost:5090/api/workloads/my-app
```

#### Create a backup
```bash
curl -X POST http://localhost:5090/api/backups \
  -H "Content-Type: application/json" \
  -d '{"name": "backup-20240206", "description": "Daily backup"}'
```

### Python

```python
import requests

# Base URL
base_url = "http://localhost:5090/api"

# List workloads
response = requests.get(f"{base_url}/workloads")
workloads = response.json()["data"]

for workload in workloads:
    print(f"Workload: {workload['name']} ({workload['runtime']})")

# Get logs
response = requests.get(f"{base_url}/workloads/my-app/logs")
logs = response.json()["data"]
print(logs)

# Stop workload
response = requests.post(f"{base_url}/workloads/my-app/stop")
print(response.json())

# Delete workload
response = requests.delete(f"{base_url}/workloads/my-app")
print(response.json())
```

### JavaScript/Node.js

```javascript
const axios = require('axios');

const baseURL = 'http://localhost:5090/api';

// List workloads
async function listWorkloads() {
  const response = await axios.get(`${baseURL}/workloads`);
  return response.data.data;
}

// Get logs
async function getLogs(name) {
  const response = await axios.get(`${baseURL}/workloads/${name}/logs`);
  return response.data.data;
}

// Stop workload
async function stopWorkload(name) {
  const response = await axios.post(`${baseURL}/workloads/${name}/stop`);
  return response.data;
}

// Delete workload
async function deleteWorkload(name) {
  const response = await axios.delete(`${baseURL}/workloads/${name}`);
  return response.data;
}

// Example usage
(async () => {
  const workloads = await listWorkloads();
  console.log('Workloads:', workloads);

  const logs = await getLogs('my-app');
  console.log('Logs:', logs);
})();
```

### Go

```go
package main

import (
    "encoding/json"
    "fmt"
    "io"
    "net/http"
)

const baseURL = "http://localhost:5090/api"

type ApiResponse struct {
    Success bool        `json:"success"`
    Data    interface{} `json:"data"`
    Error   *string     `json:"error"`
}

func listWorkloads() ([]map[string]interface{}, error) {
    resp, err := http.Get(baseURL + "/workloads")
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    body, err := io.ReadAll(resp.Body)
    if err != nil {
        return nil, err
    }

    var result ApiResponse
    json.Unmarshal(body, &result)

    workloads := result.Data.([]interface{})
    workloadMaps := make([]map[string]interface{}, len(workloads))
    for i, w := range workloads {
        workloadMaps[i] = w.(map[string]interface{})
    }

    return workloadMaps, nil
}

func main() {
    workloads, err := listWorkloads()
    if err != nil {
        panic(err)
    }

    for _, w := range workloads {
        fmt.Printf("Workload: %s (%s)\n", w["name"], w["runtime"])
    }
}
```

## Security Considerations

### Production Deployment

For production use:

1. **Enable HTTPS**: Use a reverse proxy (nginx, Caddy)
2. **Authentication**: Add authentication middleware
3. **CORS**: Configure appropriate CORS headers
4. **Rate Limiting**: Protect against abuse
5. **Firewall**: Restrict access to trusted networks

### Reverse Proxy Example (nginx)

```nginx
server {
    listen 443 ssl http2;
    server_name aether.example.com;

    ssl_certificate /etc/ssl/certs/aether.crt;
    ssl_certificate_key /etc/ssl/private/aether.key;

    location / {
        proxy_pass http://127.0.0.1:5090;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Authentication
    auth_basic "Aether Dashboard";
    auth_basic_user_file /etc/nginx/.htpasswd;
}
```

## Kubernetes Deployment

Deploy the API server in Kubernetes:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aether-api
spec:
  replicas: 2
  selector:
    matchLabels:
      app: aether-api
  template:
    metadata:
      labels:
        app: aether-api
    spec:
      containers:
      - name: aether
        image: ghcr.io/ssahani/aether:latest
        command: ["aether", "serve", "--host", "0.0.0.0", "--port", "8080"]
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: state
          mountPath: /root/.aether
      volumes:
      - name: state
        persistentVolumeClaim:
          claimName: aether-state
---
apiVersion: v1
kind: Service
metadata:
  name: aether-api
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 8080
  selector:
    app: aether-api
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: aether-state
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
```

Apply:
```bash
kubectl apply -f aether-api-deployment.yaml
```

## Docker Deployment

Run the API server in Docker:

```bash
docker run -d \
  --name aether-api \
  -p 8080:8080 \
  -v ~/.aether:/root/.aether \
  ghcr.io/ssahani/aether:latest \
  serve --host 0.0.0.0 --port 8080
```

With docker-compose:

```yaml
version: '3.8'

services:
  aether-api:
    image: ghcr.io/ssahani/aether:latest
    command: serve --host 0.0.0.0 --port 8080
    ports:
      - "8080:8080"
    volumes:
      - aether-state:/root/.aether
    restart: unless-stopped

volumes:
  aether-state:
```

Start:
```bash
docker-compose up -d
```

## Monitoring

### Prometheus Integration

The API server exposes metrics that can be scraped by Prometheus:

```bash
curl http://localhost:5090/metrics
```

See [METRICS.md](METRICS.md) for full Prometheus integration guide.

### Health Checks

Kubernetes liveness probe:
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 30
```

Readiness probe:
```yaml
readinessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
```

## Troubleshooting

### Server won't start

**Error**: `Address already in use`

**Solution**: Change the port or stop the conflicting process:
```bash
# Use different port
aether serve --port 3000

# Or find and kill the process using port 8080
lsof -ti:8080 | xargs kill
```

### Cannot connect to API

**Cause**: Firewall blocking port 8080

**Solution**:
```bash
# Allow port 8080
sudo firewall-cmd --add-port=8080/tcp --permanent
sudo firewall-cmd --reload
```

### Dashboard shows "Connection error"

**Cause**: API server not running

**Solution**: Start the server:
```bash
aether serve
```

### CORS errors in browser

**Cause**: Cross-origin requests blocked

**Solution**: Use a reverse proxy with CORS headers, or access the dashboard from the same origin.

## Best Practices

1. **State Persistence**: Mount `/root/.aether` volume for state persistence
2. **Logging**: Use `-v` flag for verbose logging
3. **Backups**: Regular automated backups via API
4. **Monitoring**: Integrate with Prometheus/Grafana
5. **Security**: Use reverse proxy for SSL/TLS in production
6. **Scaling**: Run multiple API instances behind load balancer

## Limitations

Current limitations:
- In-browser exec terminal flows are not covered by automated E2E (cluster browser route and SSO login are tested).
- Use Ingress TLS or `--tls-cert` / `--tls-key` for HTTPS in production.

Enterprise SSO is provided via **OIDC** (`AETHER_OIDC_*`) and **SAML** (`AETHER_SAML_*`). For local/CI testing, set `AETHER_MOCK_IDP=1` to embed a mock IdP (see `docs/NEXT-STEPS.md`).

## Support

For WebUI/API issues:
- GitHub Issues: https://github.com/ssahani/aether/issues
- Tag: `webui` or `api`

## Related Documentation

- [Deployment Guide](DEPLOYMENT.md)
- [Metrics Guide](METRICS.md)
- [Backup Guide](BACKUP.md)
- [Helm Chart](../helm/aether/README.md)
