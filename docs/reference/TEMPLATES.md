# Workload Templates Guide

Aether provides production-ready workload templates for common use cases. These templates follow best practices for security, scalability, and reliability.

## Overview

Templates are pre-configured YAML files that provide a starting point for deploying common workload types. They include:
- Resource specifications and limits
- Health checks and monitoring
- Security best practices
- Scaling configurations
- Network and persistence settings

## Available Templates

### 1. Web Application (`web-app.yaml`)

Production-ready web application with auto-scaling, ingress, and monitoring.

**Use Cases:**
- REST APIs
- Web frontends
- API gateways
- HTTP services

**Key Features:**
- Horizontal Pod Autoscaling (2-10 replicas)
- Ingress with TLS termination
- Health checks (liveness + readiness)
- Resource limits and requests
- ConfigMap and Secret integration
- Prometheus metrics export

**Quick Start:**
```bash
# Copy template
cp templates/web-app.yaml my-web-app.yaml

# Customize
vim my-web-app.yaml
# Update: metadata.name, image details, ingress host

# Validate
aether -s my-web-app.yaml validate

# Deploy
aether -s my-web-app.yaml run
```

**Customization Points:**
- Image: `image.repository`, `image.tag`
- Domain: `network.ingress.hosts[0].host`
- Scaling: `scaling.minReplicas`, `scaling.maxReplicas`
- Resources: `requirements.cpu`, `requirements.memory`
- Environment: `env` section

### 2. Database (`database.yaml`)

Stateful PostgreSQL database with persistence and backups.

**Use Cases:**
- Relational databases
- Data warehouses
- Analytics databases
- Transaction processing

**Key Features:**
- Persistent storage with fast SSD
- Database-specific health checks
- Backup annotations
- Resource allocation for production
- Security hardening
- Metrics export

**Quick Start:**
```bash
# Copy template
cp templates/database.yaml my-database.yaml

# Customize
vim my-database.yaml
# Update: metadata.name, storage size, credentials

# Create secrets first
kubectl create secret generic postgres-credentials \
  --from-literal=username=postgres \
  --from-literal=password=your-secure-password

# Validate
aether -s my-database.yaml validate

# Deploy
aether -s my-database.yaml run
```

**Customization Points:**
- Storage: `requirements.storage`, `persistence.storageClass`
- Database: `env.POSTGRES_DB`
- Resources: `requirements.cpu`, `requirements.memory`
- Version: `image.tag`

**Important Notes:**
- Always use secrets for credentials
- Enable regular backups
- Use fast storage (SSD/NVMe) for production
- Monitor disk usage and I/O metrics

### 3. ML Training Job (`ml-training.yaml`)

GPU-accelerated machine learning training workload.

**Use Cases:**
- Model training
- Deep learning
- Computer vision
- NLP training

**Key Features:**
- Multi-GPU support (4x NVIDIA Tesla V100)
- Large memory allocation (128Gi)
- Fast NVMe storage (1Ti)
- Distributed training configuration
- TensorBoard integration
- Experiment tracking (Weights & Biases)

**Quick Start:**
```bash
# Copy template
cp templates/ml-training.yaml my-training-job.yaml

# Customize
vim my-training-job.yaml
# Update: metadata.name, image, model config

# Create secrets
kubectl create secret generic ml-secrets \
  --from-literal=wandb-api-key=your-key \
  --from-literal=huggingface-token=your-token

# Validate
aether -s my-training-job.yaml validate

# Deploy
aether -s my-training-job.yaml run
```

**Customization Points:**
- GPUs: `requirements.gpu.count`, `requirements.gpu.type`
- Memory: `requirements.memory`
- Storage: `requirements.storage`
- Framework: `annotations.training.framework`

**Important Notes:**
- Requires GPU-enabled cluster
- Check GPU availability before deployment
- Monitor GPU utilization
- Use fast storage for dataset access
- Configure distributed training for multi-GPU

### 4. Redis Cache (`redis-cache.yaml`)

High-performance in-memory cache with persistence.

**Use Cases:**
- Session storage
- Application caching
- Message queuing
- Real-time analytics

**Key Features:**
- Persistence enabled
- Memory management (LRU eviction)
- Health checks
- Metrics export
- Password protection
- Backup support

**Quick Start:**
```bash
# Copy template
cp templates/redis-cache.yaml my-redis.yaml

# Customize
vim my-redis.yaml
# Update: metadata.name, memory limits

# Create secrets
kubectl create secret generic redis-credentials \
  --from-literal=password=your-redis-password

# Validate
aether -s my-redis.yaml validate

# Deploy
aether -s my-redis.yaml run
```

**Customization Points:**
- Memory: `env.REDIS_MAXMEMORY`, `requirements.memory`
- Eviction: `env.REDIS_MAXMEMORY_POLICY`
- Persistence: `persistence.enabled`, `persistence.storageClass`

**Important Notes:**
- Size memory appropriately (leave headroom)
- Choose eviction policy based on use case
- Enable persistence for critical data
- Monitor memory usage and evictions

### 5. Batch Job (`batch-job.yaml`)

One-time or scheduled batch processing job.

**Use Cases:**
- ETL pipelines
- Data processing
- Report generation
- Database migrations
- Backup jobs

**Key Features:**
- Retry mechanism
- Parallel processing
- TTL for cleanup
- Shared storage access
- Completion tracking
- Resource limits

**Quick Start:**
```bash
# Copy template
cp templates/batch-job.yaml my-batch-job.yaml

# Customize
vim my-batch-job.yaml
# Update: metadata.name, processing parameters

# Validate
aether -s my-batch-job.yaml validate

# Run job
aether -s my-batch-job.yaml run

# Check status
aether status my-batch-job

# View logs
aether logs my-batch-job
```

**Customization Points:**
- Parallelism: `env.PARALLEL_WORKERS`
- Batch size: `env.BATCH_SIZE`
- Retries: `annotations.batch.kubernetes.io/backoff-limit`
- Cleanup: `annotations.batch.kubernetes.io/ttl-after-finished`

**Important Notes:**
- Jobs run to completion then exit
- Configure appropriate retry limits
- Set TTL for automatic cleanup
- Use shared storage for large datasets

### 6. Microservice (`microservice.yaml`)

Cloud-native microservice with service mesh integration.

**Use Cases:**
- REST APIs
- gRPC services
- Event-driven services
- Business logic services

**Key Features:**
- Auto-scaling (3-20 replicas)
- HTTP and gRPC endpoints
- Circuit breaker configuration
- Rate limiting
- Distributed tracing (Jaeger)
- Service mesh ready (Istio/Linkerd)
- Read-only root filesystem

**Quick Start:**
```bash
# Copy template
cp templates/microservice.yaml my-service.yaml

# Customize
vim my-service.yaml
# Update: metadata.name, image, service ports

# Create secrets
kubectl create secret generic payment-secrets \
  --from-literal=stripe-api-key=sk_test_... \
  --from-file=jwt-private-key=jwt.pem

# Validate
aether -s my-service.yaml validate

# Deploy
aether -s my-service.yaml run
```

**Customization Points:**
- Scaling: `scaling.minReplicas`, `scaling.targetCPU`
- Ports: `network.ports`
- Service mesh: `annotations.sidecar.istio.io/inject`
- Tracing: `env.JAEGER_ENDPOINT`

**Important Notes:**
- Design for horizontal scaling
- Implement health check endpoints
- Use circuit breakers for external calls
- Enable distributed tracing
- Follow 12-factor app principles

## Template Structure

All templates follow this structure:

```yaml
metadata:         # Workload identification
  name: ...
  owner: ...
  project: ...
  labels: ...

image:            # Container image details
  registry: ...
  repository: ...
  tag: ...

requirements:     # Resource requirements
  cpu: ...
  memory: ...
  storage: ...
  gpu: ...

runtime:          # Runtime selection
  preferred: ...
  allowed: ...

scaling:          # Auto-scaling config
  enabled: ...
  minReplicas: ...
  maxReplicas: ...

network:          # Network configuration
  service: ...
  ports: ...
  ingress: ...

persistence:      # Storage configuration
  enabled: ...
  storageClass: ...

health:           # Health checks
  liveness: ...
  readiness: ...

env:              # Environment variables
  - name: ...
    value: ...

configMaps:       # Configuration files
  - name: ...
    mountPath: ...

secrets:          # Sensitive data
  - name: ...
    mountPath: ...

resources:        # Resource limits
  limits: ...
  requests: ...

security:         # Security settings
  runAsNonRoot: ...
  capabilities: ...

monitoring:       # Monitoring config
  enabled: ...
  path: ...

annotations:      # Additional metadata
  key: value
```

## Best Practices

### Security

1. **Run as Non-Root**
   ```yaml
   security:
     runAsNonRoot: true
     runAsUser: 1000
     fsGroup: 1000
   ```

2. **Drop Capabilities**
   ```yaml
   security:
     capabilities:
       drop:
         - ALL
   ```

3. **Read-Only Filesystem** (when possible)
   ```yaml
   security:
     readOnlyRootFilesystem: true
   ```

4. **Use Secrets for Sensitive Data**
   ```yaml
   env:
     - name: API_KEY
       valueFrom:
         secretRef:
           name: api-secrets
           key: key
   ```

### Resource Management

1. **Set Resource Limits**
   ```yaml
   resources:
     limits:
       cpu: "2"
       memory: "4Gi"
     requests:
       cpu: "1"
       memory: "2Gi"
   ```

2. **Request Less Than Limit**
   - Requests: Normal usage
   - Limits: Maximum allowed

3. **Monitor Usage**
   - Use Prometheus metrics
   - Set up alerts for high usage

### Health Checks

1. **Implement Health Endpoints**
   ```bash
   GET /health/live   # Liveness probe
   GET /health/ready  # Readiness probe
   ```

2. **Configure Probes**
   ```yaml
   health:
     liveness:
       enabled: true
       path: /health/live
       initialDelay: 30
       period: 10
     readiness:
       enabled: true
       path: /health/ready
       initialDelay: 10
       period: 5
   ```

### Scaling

1. **Enable HPA for Variable Load**
   ```yaml
   scaling:
     enabled: true
     minReplicas: 2
     maxReplicas: 10
     targetCPU: 70
   ```

2. **Run Multiple Replicas**
   - Minimum 2 for high availability
   - Spread across availability zones

3. **Use PodDisruptionBudget**
   ```yaml
   annotations:
     policy.kubernetes.io/min-available: "1"
   ```

### Networking

1. **Use Services for Communication**
   ```yaml
   network:
     service: true
     serviceType: ClusterIP
   ```

2. **Enable Ingress for External Access**
   ```yaml
   network:
     ingress:
       enabled: true
       hosts:
         - host: app.example.com
   ```

3. **Configure TLS**
   ```yaml
   network:
     ingress:
       tls:
         - secretName: tls-cert
           hosts:
             - app.example.com
   ```

### Persistence

1. **Use Appropriate Storage Class**
   - Fast SSD for databases
   - Standard for logs/backups
   - NVMe for ML training

2. **Set Correct Access Mode**
   - ReadWriteOnce: Single pod
   - ReadWriteMany: Multiple pods

3. **Enable Backups**
   ```yaml
   annotations:
     backup.velero.io/backup-volumes: "data"
     backup.schedule: "0 2 * * *"
   ```

## Customization Workflow

### 1. Copy Template
```bash
cp templates/web-app.yaml my-application.yaml
```

### 2. Update Metadata
```yaml
metadata:
  name: my-application
  owner: my-team
  project: my-project
```

### 3. Configure Image
```yaml
image:
  registry: ghcr.io
  repository: myorg/my-app
  tag: "1.0.0"
```

### 4. Set Resources
```yaml
requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "50Gi"
```

### 5. Configure Networking
```yaml
network:
  ingress:
    hosts:
      - host: my-app.example.com
```

### 6. Add Environment Variables
```yaml
env:
  - name: APP_NAME
    value: my-application
  - name: DATABASE_URL
    valueFrom:
      secretRef:
        name: db-credentials
        key: url
```

### 7. Validate
```bash
aether -s my-application.yaml validate
```

### 8. Deploy
```bash
aether -s my-application.yaml run
```

## Creating Custom Templates

### Template Variables

Use descriptive names that need replacement:
```yaml
metadata:
  name: REPLACE_WITH_YOUR_NAME
  owner: REPLACE_WITH_YOUR_TEAM

image:
  repository: YOUR_ORG/YOUR_APP
  tag: "VERSION"
```

### Include Comments
```yaml
# Set this to your application domain
network:
  ingress:
    hosts:
      - host: app.example.com  # Update this

# Configure based on load testing results
scaling:
  minReplicas: 2  # Minimum for HA
  maxReplicas: 10  # Adjust based on capacity
```

### Document Requirements
Add a header comment:
```yaml
# Custom Application Template
#
# Prerequisites:
# - PostgreSQL database
# - Redis cache
# - S3 bucket for uploads
#
# Required Secrets:
# - db-credentials (username, password, url)
# - api-keys (stripe-key, aws-key)
#
# Customization:
# 1. Update metadata.name
# 2. Set image.repository and image.tag
# 3. Configure network.ingress.hosts
# 4. Adjust resources based on load
```

## Template Validation

### Common Issues

1. **Invalid Resource Format**
   ```yaml
   # ❌ Wrong
   cpu: 2

   # ✅ Correct
   cpu: "2"
   ```

2. **Missing Required Fields**
   ```yaml
   # ❌ Missing name
   metadata:
     owner: team

   # ✅ Complete
   metadata:
     name: my-app
     owner: team
   ```

3. **Invalid References**
   ```yaml
   # ❌ Secret doesn't exist
   env:
     - name: KEY
       valueFrom:
         secretRef:
           name: nonexistent-secret

   # ✅ Create secret first
   # kubectl create secret generic my-secret --from-literal=KEY=value
   ```

### Validation Commands

```bash
# Validate syntax
aether -s template.yaml validate

# Dry-run (if supported)
kubectl apply -f template.yaml --dry-run=client

# Cost estimate
aether -s template.yaml cost

# Schema validation
yq eval template.yaml
```

## Version Control

### Store Templates in Git

```bash
# Initialize repository
git init aether-templates
cd aether-templates

# Organize by category
mkdir -p templates/{web,database,ml,batch,microservices}

# Add templates
cp /path/to/web-app.yaml templates/web/
git add templates/
git commit -m "Add web app template"

# Tag versions
git tag -a v1.0.0 -m "Initial template collection"
```

### Template Versioning

```yaml
metadata:
  name: web-app
  labels:
    template.version: "2.0.0"
    template.maintainer: platform-team
  annotations:
    template.changelog: "Added ingress support, updated security settings"
```

## Support

For template-related issues:
- GitHub Issues: https://github.com/ssahani/aether/issues
- Tag: `template`
- Include: Template name, error message, YAML content

## Related Documentation

- [Workload Specification](../README.md#workload-specification)
- [Deployment Guide](DEPLOYMENT.md)
- [Security Best Practices](../README.md#security)
- [Cost Estimation](COST.md)
