# Microservices E-Commerce Example

Complete e-commerce application demonstrating Orchestr8 multi-runtime deployment with microservices architecture.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Load Balancer (Ingress)                │
└─────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      API Gateway Service                     │
│                     (Kubernetes - 3 replicas)                │
└─────────────────────────────────────────────────────────────┘
          ▼              ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
    │  User    │   │ Product  │   │  Order   │   │ Payment  │
    │ Service  │   │ Service  │   │ Service  │   │ Service  │
    │ (K8s)    │   │ (K8s)    │   │ (K8s)    │   │ (K8s)    │
    │ 2-5      │   │ 2-10     │   │ 3-8      │   │ 3-10     │
    └──────────┘   └──────────┘   └──────────┘   └──────────┘
          ▼              ▼              ▼              ▼
    ┌──────────────────────────────────────────────────────────┐
    │               PostgreSQL Database (K8s)                   │
    │               - 500Gi SSD Storage                         │
    │               - 16Gi Memory                               │
    └──────────────────────────────────────────────────────────┘
          ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐
    │  Redis   │   │  RabbitMQ│   │  Elastic │
    │  Cache   │   │  Queue   │   │  Search  │
    │ (K8s)    │   │ (K8s)    │   │ (K8s)    │
    └──────────┘   └──────────┘   └──────────┘
```

## Services

### 1. API Gateway
- **Runtime:** Kubernetes
- **Replicas:** 3-10 (auto-scaling)
- **Resources:** 1 CPU, 2Gi memory
- **Purpose:** Route requests, authentication, rate limiting

### 2. User Service
- **Runtime:** Kubernetes
- **Replicas:** 2-5
- **Resources:** 500m CPU, 1Gi memory
- **Features:** User management, authentication, profiles

### 3. Product Service
- **Runtime:** Kubernetes
- **Replicas:** 2-10
- **Resources:** 1 CPU, 2Gi memory
- **Features:** Product catalog, inventory, search

### 4. Order Service
- **Runtime:** Kubernetes
- **Replicas:** 3-8
- **Resources:** 1 CPU, 2Gi memory
- **Features:** Order management, fulfillment tracking

### 5. Payment Service
- **Runtime:** Kubernetes
- **Replicas:** 3-10
- **Resources:** 1 CPU, 2Gi memory
- **Features:** Payment processing, Stripe integration

### 6. PostgreSQL Database
- **Runtime:** Kubernetes
- **Storage:** 500Gi SSD
- **Resources:** 4 CPU, 16Gi memory
- **Backups:** Daily automated backups

### 7. Redis Cache
- **Runtime:** Kubernetes
- **Storage:** 100Gi
- **Resources:** 2 CPU, 8Gi memory
- **Purpose:** Session storage, caching

### 8. RabbitMQ
- **Runtime:** Kubernetes
- **Replicas:** 3 (HA cluster)
- **Resources:** 1 CPU, 2Gi memory
- **Purpose:** Event-driven communication

### 9. Elasticsearch
- **Runtime:** Kubernetes
- **Replicas:** 3 (cluster)
- **Storage:** 1Ti
- **Resources:** 2 CPU, 8Gi memory
- **Purpose:** Product search, analytics

## Deployment

### Prerequisites

```bash
# Install Orchestr8
curl -LO https://github.com/ssahani/orchestr8/releases/latest/download/orchestr8-linux-amd64
chmod +x orchestr8-linux-amd64
sudo mv orchestr8-linux-amd64 /usr/local/bin/orchestr8

# Configure Kubernetes access
export KUBECONFIG=~/.kube/config

# Create namespace
kubectl create namespace ecommerce
```

### Deploy Infrastructure

```bash
# 1. Deploy database
orchestr8 -s infrastructure/postgres.yaml run

# 2. Deploy cache
orchestr8 -s infrastructure/redis.yaml run

# 3. Deploy message queue
orchestr8 -s infrastructure/rabbitmq.yaml run

# 4. Deploy search
orchestr8 -s infrastructure/elasticsearch.yaml run

# Wait for infrastructure
kubectl wait --for=condition=ready pod -l tier=infrastructure -n ecommerce --timeout=600s
```

### Deploy Services

```bash
# Deploy all microservices
for service in services/*.yaml; do
  echo "Deploying $service..."
  orchestr8 -s "$service" run
  sleep 5
done

# Verify deployments
orchestr8 list
kubectl get pods -n ecommerce
```

### Configure Ingress

```bash
# Deploy API Gateway
orchestr8 -s api-gateway.yaml run

# Configure ingress
kubectl apply -f ingress.yaml

# Get ingress URL
kubectl get ingress -n ecommerce
```

## Configuration

### Secrets

Create required secrets:

```bash
# Database credentials
kubectl create secret generic postgres-credentials \
  --from-literal=username=ecommerce \
  --from-literal=password=$(openssl rand -base64 32) \
  -n ecommerce

# Redis password
kubectl create secret generic redis-credentials \
  --from-literal=password=$(openssl rand -base64 32) \
  -n ecommerce

# Payment provider keys
kubectl create secret generic payment-secrets \
  --from-literal=stripe-api-key=$STRIPE_API_KEY \
  --from-literal=stripe-webhook-secret=$STRIPE_WEBHOOK_SECRET \
  -n ecommerce

# JWT signing key
kubectl create secret generic jwt-secrets \
  --from-file=private-key=jwt-private.pem \
  --from-file=public-key=jwt-public.pem \
  -n ecommerce
```

### ConfigMaps

```bash
# Service configuration
kubectl create configmap service-config \
  --from-file=config/services.yaml \
  -n ecommerce

# Feature flags
kubectl create configmap feature-flags \
  --from-literal=enable-recommendations=true \
  --from-literal=enable-reviews=true \
  -n ecommerce
```

## Monitoring

### Prometheus Metrics

All services expose metrics at `/metrics`:

```bash
# Check metrics
curl http://user-service:9090/metrics
curl http://product-service:9090/metrics
curl http://order-service:9090/metrics
```

### Grafana Dashboard

Import the provided dashboard:

```bash
# Import dashboard
kubectl apply -f monitoring/grafana-dashboard.yaml
```

### Alerts

Configure alerts:

```bash
kubectl apply -f monitoring/prometheus-rules.yaml
```

## Scaling

### Auto-Scaling

Services are configured with HPA:

```bash
# Check HPA status
kubectl get hpa -n ecommerce

# Manually scale
kubectl scale deployment product-service --replicas=5 -n ecommerce
```

### Load Testing

```bash
# Install k6
brew install k6  # or download from k6.io

# Run load test
k6 run load-tests/checkout-flow.js

# Monitor during test
watch -n 1 kubectl get hpa -n ecommerce
```

## Cost Estimation

Estimate monthly costs:

```bash
# Per service
for service in services/*.yaml infrastructure/*.yaml; do
  echo "=== $service ==="
  orchestr8 -s "$service" cost --provider aws
done

# Total estimate
./scripts/calculate-total-cost.sh
```

Expected monthly costs (AWS us-east-1):
- Infrastructure: ~$500/month
- Microservices: ~$800/month
- **Total: ~$1,300/month**

## Backup & Recovery

### Database Backups

```bash
# Create backup
orchestr8 backup -n "ecommerce-$(date +%Y%m%d)" \
  -d "Daily backup of e-commerce platform"

# Schedule daily backups
kubectl apply -f backup/cronjob.yaml
```

### Disaster Recovery

```bash
# Full system backup
./scripts/backup-all.sh

# Restore from backup
./scripts/restore-all.sh ecommerce-20240206.tar.gz
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy E-Commerce

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Deploy services
        run: |
          for service in services/*.yaml; do
            orchestr8 -s "$service" run
          done
```

### GitLab CI

```yaml
# .gitlab-ci.yml
deploy:production:
  stage: deploy
  script:
    - cd examples/microservices
    - ./scripts/deploy-all.sh
  only:
    - main
```

## Testing

### Integration Tests

```bash
# Install dependencies
npm install

# Run integration tests
npm run test:integration

# Run E2E tests
npm run test:e2e
```

### Smoke Tests

```bash
# Health checks
./scripts/smoke-test.sh

# Expected output:
# ✅ API Gateway: healthy
# ✅ User Service: healthy
# ✅ Product Service: healthy
# ✅ Order Service: healthy
# ✅ Payment Service: healthy
```

## Performance

### Expected Performance

- **Throughput:** 10,000 requests/second
- **Latency (p95):** <200ms
- **Latency (p99):** <500ms
- **Uptime:** 99.9%

### Optimization

```bash
# Enable caching
kubectl set env deployment/product-service CACHE_ENABLED=true

# Adjust replica counts
kubectl scale deployment/product-service --replicas=10

# Monitor performance
kubectl top pods -n ecommerce
```

## Troubleshooting

### Service Not Starting

```bash
# Check pod status
kubectl get pods -n ecommerce -l app=user-service

# View logs
orchestr8 logs user-service

# Describe pod
kubectl describe pod -n ecommerce -l app=user-service
```

### Database Connection Issues

```bash
# Test connectivity
kubectl run -it --rm debug \
  --image=postgres:16 \
  --restart=Never \
  -- psql -h postgres -U ecommerce

# Check secrets
kubectl get secret postgres-credentials -n ecommerce -o yaml
```

### High Latency

```bash
# Check resource usage
kubectl top pods -n ecommerce

# Scale up services
kubectl scale deployment/product-service --replicas=8

# Check database performance
kubectl exec -it postgres-0 -n ecommerce -- \
  psql -U ecommerce -c "SELECT * FROM pg_stat_activity;"
```

## Migration

### Runtime Migration

Migrate services between runtimes:

```bash
# Migrate to different runtime
orchestr8 migrate product-service kubernetes \
  --strategy blue-green

# Verify migration
orchestr8 status product-service
```

## Security

### Network Policies

```bash
# Apply network policies
kubectl apply -f security/network-policies.yaml

# Verify policies
kubectl get networkpolicies -n ecommerce
```

### Pod Security

All services run with:
- Non-root user (UID 1000)
- Dropped capabilities
- Read-only root filesystem (where possible)
- Seccomp profile

### TLS/SSL

```bash
# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Create certificate
kubectl apply -f security/certificate.yaml
```

## Cleanup

```bash
# Delete all services
for service in services/*.yaml infrastructure/*.yaml; do
  orchestr8 delete $(basename "$service" .yaml)
done

# Or use script
./scripts/cleanup.sh

# Delete namespace
kubectl delete namespace ecommerce
```

## Documentation

- [Architecture Design](docs/architecture.md)
- [API Documentation](docs/api.md)
- [Deployment Guide](docs/deployment.md)
- [Troubleshooting](docs/troubleshooting.md)

## Support

For issues or questions:
- GitHub Issues: https://github.com/ssahani/orchestr8/issues
- Tag: `example-microservices`
