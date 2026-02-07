# Orchestr8 Workload Templates

Production-ready templates for common workload types.

## Quick Reference

| Template | Use Case | Runtime | Scaling | Storage |
|----------|----------|---------|---------|---------|
| `web-app.yaml` | Web applications, APIs | Kubernetes | ✅ HPA (2-10) | 50Gi |
| `database.yaml` | PostgreSQL database | Kubernetes | ❌ Stateful | 500Gi |
| `ml-training.yaml` | ML model training | KubeVirt | ❌ GPU job | 1Ti |
| `redis-cache.yaml` | In-memory cache | Kubernetes | ❌ Single | 100Gi |
| `batch-job.yaml` | Batch processing | Kubernetes | ❌ Job | 200Gi |
| `microservice.yaml` | Microservices | Kubernetes | ✅ HPA (3-20) | None |

## Quick Start

### 1. Choose a Template

```bash
ls templates/
```

### 2. Copy and Customize

```bash
cp templates/web-app.yaml my-app.yaml
vim my-app.yaml
```

### 3. Validate

```bash
orchestr8 -s my-app.yaml validate
```

### 4. Deploy

```bash
orchestr8 -s my-app.yaml run
```

## Template Details

### Web Application

**File:** `web-app.yaml`

**Features:**
- Auto-scaling (2-10 replicas)
- Ingress with TLS
- Health checks
- ConfigMaps and Secrets
- Resource limits

**Customize:**
- `metadata.name` - Your app name
- `image.repository` - Your image
- `network.ingress.hosts[0].host` - Your domain
- `requirements.cpu/memory` - Your resources

**Example:**
```bash
cp templates/web-app.yaml production-api.yaml
# Edit: name, image, domain, resources
orchestr8 -s production-api.yaml run
```

### Database (PostgreSQL)

**File:** `database.yaml`

**Features:**
- Persistent storage (500Gi)
- Database health checks
- Backup annotations
- Security hardening
- Metrics export

**Prerequisites:**
```bash
kubectl create secret generic postgres-credentials \
  --from-literal=username=postgres \
  --from-literal=password=SECURE_PASSWORD
```

**Customize:**
- `metadata.name` - Database name
- `requirements.storage` - Disk size
- `env.POSTGRES_DB` - Database name

**Example:**
```bash
cp templates/database.yaml app-database.yaml
# Edit: name, storage, database
orchestr8 -s app-database.yaml run
```

### ML Training Job

**File:** `ml-training.yaml`

**Features:**
- 4x NVIDIA Tesla V100 GPUs
- 128Gi memory
- 1Ti fast storage
- Distributed training
- TensorBoard integration

**Prerequisites:**
```bash
kubectl create secret generic ml-secrets \
  --from-literal=wandb-api-key=YOUR_KEY \
  --from-literal=huggingface-token=YOUR_TOKEN
```

**Customize:**
- `requirements.gpu.count` - Number of GPUs
- `requirements.memory` - Memory size
- `image.repository` - Training image

**Example:**
```bash
cp templates/ml-training.yaml bert-training.yaml
# Edit: name, GPUs, model config
orchestr8 -s bert-training.yaml run
```

### Redis Cache

**File:** `redis-cache.yaml`

**Features:**
- In-memory caching
- LRU eviction policy
- Persistence enabled
- Password protection
- Metrics export

**Prerequisites:**
```bash
kubectl create secret generic redis-credentials \
  --from-literal=password=REDIS_PASSWORD
```

**Customize:**
- `env.REDIS_MAXMEMORY` - Max memory
- `env.REDIS_MAXMEMORY_POLICY` - Eviction policy
- `persistence.enabled` - Enable/disable persistence

**Example:**
```bash
cp templates/redis-cache.yaml session-cache.yaml
# Edit: name, memory limit
orchestr8 -s session-cache.yaml run
```

### Batch Job

**File:** `batch-job.yaml`

**Features:**
- Retry mechanism (3 attempts)
- Parallel processing (16 workers)
- Automatic cleanup (1h TTL)
- Shared storage
- Metrics

**Customize:**
- `env.BATCH_SIZE` - Batch size
- `env.PARALLEL_WORKERS` - Worker count
- `annotations.batch.kubernetes.io/backoff-limit` - Retries

**Example:**
```bash
cp templates/batch-job.yaml data-import.yaml
# Edit: name, batch size, workers
orchestr8 -s data-import.yaml run
```

### Microservice

**File:** `microservice.yaml`

**Features:**
- Auto-scaling (3-20 replicas)
- HTTP + gRPC endpoints
- Service mesh ready
- Circuit breaker config
- Distributed tracing

**Prerequisites:**
```bash
kubectl create secret generic payment-secrets \
  --from-literal=stripe-api-key=sk_test_...
```

**Customize:**
- `network.ports` - Service ports
- `scaling.minReplicas` - Min replicas
- `env` - Service configuration

**Example:**
```bash
cp templates/microservice.yaml user-service.yaml
# Edit: name, ports, scaling
orchestr8 -s user-service.yaml run
```

## Best Practices

### Security
- ✅ Always use secrets for credentials
- ✅ Run as non-root user
- ✅ Drop all capabilities
- ✅ Use read-only root filesystem (when possible)

### Resources
- ✅ Set both requests and limits
- ✅ Request less than limit
- ✅ Monitor actual usage
- ✅ Adjust based on metrics

### Scaling
- ✅ Run at least 2 replicas for HA
- ✅ Set appropriate HPA targets
- ✅ Test scaling behavior
- ✅ Configure PodDisruptionBudget

### Storage
- ✅ Use fast storage (SSD) for databases
- ✅ Enable backups for stateful workloads
- ✅ Set appropriate storage class
- ✅ Monitor disk usage

### Monitoring
- ✅ Enable Prometheus metrics
- ✅ Implement health check endpoints
- ✅ Configure alerts
- ✅ Use distributed tracing

## Common Workflows

### Development Workflow
```bash
# 1. Copy template
cp templates/web-app.yaml dev-app.yaml

# 2. Update for development
#    - Reduce replicas (min: 1, max: 2)
#    - Lower resource limits
#    - Disable TLS
#    - Use development image tag

# 3. Validate
orchestr8 -s dev-app.yaml validate

# 4. Deploy
orchestr8 -s dev-app.yaml run
```

### Production Workflow
```bash
# 1. Copy template
cp templates/web-app.yaml prod-app.yaml

# 2. Production configuration
#    - Set production domain
#    - Enable TLS
#    - Set proper resource limits
#    - Configure monitoring
#    - Enable backups

# 3. Cost estimate
orchestr8 -s prod-app.yaml cost

# 4. Validate
orchestr8 -s prod-app.yaml validate

# 5. Create backup
orchestr8 backup -n pre-deployment

# 6. Deploy
orchestr8 -s prod-app.yaml run

# 7. Verify
orchestr8 status prod-app
orchestr8 logs prod-app
```

### Migration Workflow
```bash
# 1. Deploy to test environment
orchestr8 -s app.yaml run

# 2. Validate functionality
orchestr8 status app
curl https://test.example.com/health

# 3. Create backup
orchestr8 backup -n pre-migration

# 4. Migrate to production runtime
orchestr8 migrate app kubernetes --strategy blue-green

# 5. Verify migration
orchestr8 status app
curl https://prod.example.com/health
```

## Troubleshooting

### Template Validation Fails

**Issue:** `Invalid resource format`
```bash
Error: cpu value must be a string
```

**Solution:**
```yaml
# ❌ Wrong
requirements:
  cpu: 2

# ✅ Correct
requirements:
  cpu: "2"
```

### Secret Not Found

**Issue:** `secret "db-credentials" not found`

**Solution:**
```bash
# Create the secret first
kubectl create secret generic db-credentials \
  --from-literal=username=postgres \
  --from-literal=password=your-password
```

### Insufficient Resources

**Issue:** `Insufficient CPU/memory`

**Solution:**
```yaml
# Reduce resource requests
resources:
  requests:
    cpu: "500m"    # Was: "2"
    memory: "1Gi"  # Was: "4Gi"
```

### Image Pull Failed

**Issue:** `Failed to pull image`

**Solution:**
```yaml
# Check image name and tag
image:
  registry: ghcr.io
  repository: myorg/myapp  # Verify this exists
  tag: "1.0.0"             # Verify tag exists

# Or add image pull secrets
imagePullSecrets:
  - name: regcred
```

## Documentation

- [Complete Template Guide](../docs/TEMPLATES.md)
- [Workload Specification](../README.md)
- [Deployment Guide](../docs/DEPLOYMENT.md)
- [Cost Estimation](../docs/COST.md)

## Contributing

To add a new template:

1. Create YAML file in `templates/`
2. Follow existing template structure
3. Add comprehensive comments
4. Include example values
5. Document prerequisites
6. Add to this README
7. Test deployment
8. Submit PR

## Support

Issues? Questions?
- GitHub: https://github.com/ssahani/orchestr8/issues
- Tag: `template`

## License

Same as Orchestr8 project (MIT OR Apache-2.0)
