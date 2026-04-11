# WebUI and REST API Guide

Orchestr8 provides a modern web dashboard and REST API for managing workloads through your browser.

## Overview

The WebUI provides:
- **Real-time Dashboard**: Monitor all workloads across runtimes
- **REST API**: Programmatic access to all operations
- **Live Statistics**: Track workload counts, runtimes, and backups
- **Log Viewing**: Browse container logs in your browser
- **Workload Management**: Stop and delete workloads with one click

## Quick Start

### Start the Server

```bash
orchestr8 serve
```

The server starts on `http://127.0.0.1:8080` by default.

### Custom Host and Port

```bash
orchestr8 serve --host 0.0.0.0 --port 3000
```

### Access the Dashboard

Open your browser and navigate to:
```
http://localhost:8080
```

## Dashboard Features

### Statistics Overview

The dashboard displays four key metrics:
- **Total Workloads**: Number of deployed workloads
- **Running**: Count of running instances
- **Runtimes**: Number of unique runtimes in use
- **Backups**: Available backup snapshots

### Workload Table

View all workloads with:
- **Name**: Workload identifier
- **Runtime**: Deployment target (Podman, Kubernetes, KubeVirt, Metal3)
- **Image**: Container or VM image
- **Status**: Current state (running, stopped)
- **Created**: Deployment timestamp
- **Actions**: Quick access buttons

### Actions

For each workload:
- **📋 Logs**: View recent logs in a modal dialog
- **⏸️ Stop**: Stop the running workload
- **🗑️ Delete**: Remove the workload completely

### Auto-Refresh

The dashboard automatically refreshes every 5 seconds to show the latest state.

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
    "/home/user/.orchestr8/backups/backup-20240206-143052.json",
    "/home/user/.orchestr8/backups/production-2024-02-06.json"
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
  "data": "Backup created: /home/user/.orchestr8/backups/my-backup.json"
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
curl http://localhost:8080/api/workloads
```

#### Get workload logs
```bash
curl http://localhost:8080/api/workloads/my-app/logs
```

#### Stop a workload
```bash
curl -X POST http://localhost:8080/api/workloads/my-app/stop
```

#### Delete a workload
```bash
curl -X DELETE http://localhost:8080/api/workloads/my-app
```

#### Create a backup
```bash
curl -X POST http://localhost:8080/api/backups \
  -H "Content-Type: application/json" \
  -d '{"name": "backup-20240206", "description": "Daily backup"}'
```

### Python

```python
import requests

# Base URL
base_url = "http://localhost:8080/api"

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

const baseURL = 'http://localhost:8080/api';

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

const baseURL = "http://localhost:8080/api"

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
    server_name orchestr8.example.com;

    ssl_certificate /etc/ssl/certs/orchestr8.crt;
    ssl_certificate_key /etc/ssl/private/orchestr8.key;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Authentication
    auth_basic "Orchestr8 Dashboard";
    auth_basic_user_file /etc/nginx/.htpasswd;
}
```

## Kubernetes Deployment

Deploy the API server in Kubernetes:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: orchestr8-api
spec:
  replicas: 2
  selector:
    matchLabels:
      app: orchestr8-api
  template:
    metadata:
      labels:
        app: orchestr8-api
    spec:
      containers:
      - name: orchestr8
        image: ghcr.io/ssahani/orchestr8:latest
        command: ["orchestr8", "serve", "--host", "0.0.0.0", "--port", "8080"]
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: state
          mountPath: /root/.orchestr8
      volumes:
      - name: state
        persistentVolumeClaim:
          claimName: orchestr8-state
---
apiVersion: v1
kind: Service
metadata:
  name: orchestr8-api
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 8080
  selector:
    app: orchestr8-api
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: orchestr8-state
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
```

Apply:
```bash
kubectl apply -f orchestr8-api-deployment.yaml
```

## Docker Deployment

Run the API server in Docker:

```bash
docker run -d \
  --name orchestr8-api \
  -p 8080:8080 \
  -v ~/.orchestr8:/root/.orchestr8 \
  ghcr.io/ssahani/orchestr8:latest \
  serve --host 0.0.0.0 --port 8080
```

With docker-compose:

```yaml
version: '3.8'

services:
  orchestr8-api:
    image: ghcr.io/ssahani/orchestr8:latest
    command: serve --host 0.0.0.0 --port 8080
    ports:
      - "8080:8080"
    volumes:
      - orchestr8-state:/root/.orchestr8
    restart: unless-stopped

volumes:
  orchestr8-state:
```

Start:
```bash
docker-compose up -d
```

## Monitoring

### Prometheus Integration

The API server exposes metrics that can be scraped by Prometheus:

```bash
curl http://localhost:8080/metrics
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
orchestr8 serve --port 3000

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
orchestr8 serve
```

### CORS errors in browser

**Cause**: Cross-origin requests blocked

**Solution**: Use a reverse proxy with CORS headers, or access the dashboard from the same origin.

## Best Practices

1. **State Persistence**: Mount `/root/.orchestr8` volume for state persistence
2. **Logging**: Use `-v` flag for verbose logging
3. **Backups**: Regular automated backups via API
4. **Monitoring**: Integrate with Prometheus/Grafana
5. **Security**: Use reverse proxy for SSL/TLS in production
6. **Scaling**: Run multiple API instances behind load balancer

## Limitations

Current limitations:
- No authentication/authorization built-in
- No HTTPS support (use reverse proxy)
- No WebSocket support for real-time updates
- Create workload requires full spec (no form builder yet)

Future enhancements planned:
- Built-in authentication (API keys, OAuth)
- WebSocket for live updates
- Form-based workload creation
- Metrics dashboard integration
- Multi-user support with RBAC

## Support

For WebUI/API issues:
- GitHub Issues: https://github.com/ssahani/orchestr8/issues
- Tag: `webui` or `api`

## Related Documentation

- [Deployment Guide](DEPLOYMENT.md)
- [Metrics Guide](METRICS.md)
- [Backup Guide](BACKUP.md)
- [Helm Chart](../helm/orchestr8/README.md)
