# CI/CD Integration Guide

Aether integrates seamlessly with popular CI/CD platforms for automated deployments, cost analysis, and workflow orchestration.

## Overview

This guide covers:
- CI/CD platform integration (GitHub Actions, GitLab CI, Jenkins)
- Deployment automation workflows
- Cost estimation in pipelines
- Backup and rollback strategies
- Best practices and security

## Quick Start

### Basic Deployment Pipeline

```bash
# 1. Validate workload specification
aether -s workload.yaml validate

# 2. Estimate costs
aether -s workload.yaml cost --provider aws

# 3. Create backup before deployment
aether backup -n pre-deploy-$(date +%Y%m%d)

# 4. Deploy workload
aether -s workload.yaml run --runtime kubernetes

# 5. Verify deployment
aether status my-app

# 6. Check health
curl -f https://app.example.com/health
```

## GitHub Actions

### Complete Example

See [`examples/cicd/github-actions.yml`](../examples/cicd/github-actions.yml) for a full production pipeline.

### Key Features

**Multi-Stage Pipeline:**
1. Validate workload specification
2. Estimate deployment costs
3. Build container image
4. Create pre-deployment backup
5. Deploy to staging
6. Run integration tests
7. Deploy to production
8. Automatic rollback on failure

**Cost Analysis Integration:**
```yaml
- name: Estimate deployment costs
  run: |
    aether -s $WORKLOAD_SPEC cost --provider all | tee cost-report.txt

- name: Comment cost on PR
  if: github.event_name == 'pull_request'
  uses: actions/github-script@v7
  with:
    script: |
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: '## 💰 Cost Estimation\n\n```\n${{ steps.cost.outputs.cost_report }}\n```'
      })
```

**Blue-Green Deployment:**
```yaml
- name: Deploy with blue-green strategy
  run: |
    aether migrate myapp kubernetes --strategy blue-green
```

**Automatic Rollback:**
```yaml
- name: Rollback on failure
  if: failure()
  run: |
    LATEST_BACKUP=$(aether list-backups | grep pre-deploy | head -1 | awk '{print $2}')
    aether restore $LATEST_BACKUP
```

### Secrets Configuration

Required secrets in GitHub repository settings:
- `KUBE_CONFIG_STAGING` - Kubernetes config for staging (base64 encoded)
- `KUBE_CONFIG_PRODUCTION` - Kubernetes config for production (base64 encoded)
- `GITHUB_TOKEN` - Automatically provided by GitHub

### Workflow Triggers

```yaml
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]
  workflow_dispatch:
    inputs:
      environment:
        type: choice
        options:
          - staging
          - production
```

## GitLab CI

### Complete Example

See [`examples/cicd/gitlab-ci.yml`](../examples/cicd/gitlab-ci.yml) for a full production pipeline.

### Key Features

**Multi-Stage Pipeline:**
```yaml
stages:
  - validate
  - build
  - test
  - deploy-staging
  - deploy-production
  - cleanup
```

**Review Apps:**
```yaml
review:start:
  environment:
    name: review/$CI_COMMIT_REF_SLUG
    url: https://$CI_COMMIT_REF_SLUG.review.example.com
    on_stop: review:stop
    auto_stop_in: 1 week
  script:
    - yq eval ".metadata.name = \"app-$CI_COMMIT_REF_SLUG\"" -i $WORKLOAD_SPEC
    - aether -s $WORKLOAD_SPEC run --runtime kubernetes
  only:
    - merge_requests
```

**Parallel Validation:**
```yaml
validate:spec:
  script:
    - aether -s $WORKLOAD_SPEC validate

cost:estimate:
  script:
    - aether -s $WORKLOAD_SPEC cost --provider all
```

**Scheduled Jobs:**
```yaml
cleanup:backups:
  script:
    - find ~/.aether/backups -name "*.json" -mtime +7 -delete
  only:
    - schedules
```

### Variables Configuration

Set in GitLab CI/CD settings:
- `KUBE_CONFIG_STAGING` - Kubernetes config (base64)
- `KUBE_CONFIG_PRODUCTION` - Kubernetes config (base64)
- `CI_REGISTRY_USER` - Container registry username
- `CI_REGISTRY_PASSWORD` - Container registry password

### Environment Configuration

```yaml
environment:
  name: production
  url: https://app.example.com
  on_stop: rollback:production
```

## Jenkins

### Complete Example

See [`examples/cicd/Jenkinsfile`](../examples/cicd/Jenkinsfile) for a full production pipeline.

### Key Features

**Declarative Pipeline:**
```groovy
pipeline {
    agent any

    environment {
        AETHER_VERSION = '0.1.0'
        WORKLOAD_SPEC = 'workloads/app.yaml'
    }

    stages {
        stage('Validate') { }
        stage('Build') { }
        stage('Deploy') { }
    }
}
```

**Parallel Execution:**
```groovy
stage('Validate & Analyze') {
    parallel {
        stage('Validate Workload Spec') {
            steps {
                sh 'aether -s ${WORKLOAD_SPEC} validate'
            }
        }
        stage('Cost Estimation') {
            steps {
                sh 'aether -s ${WORKLOAD_SPEC} cost --provider all'
            }
        }
    }
}
```

**Manual Approval:**
```groovy
stage('Deploy to Production') {
    steps {
        input message: 'Deploy to production?', ok: 'Deploy', submitter: 'admin,devops'
        sh 'aether -s ${WORKLOAD_SPEC} run --runtime kubernetes'
    }
}
```

**Post-Build Actions:**
```groovy
post {
    success {
        slackSend color: 'good', message: "Deployment successful"
    }
    failure {
        sh '''
            LATEST_BACKUP=$(aether list-backups | head -1)
            aether restore $LATEST_BACKUP
        '''
        slackSend color: 'danger', message: "Deployment failed - rolled back"
    }
}
```

### Credentials Configuration

Jenkins credentials needed:
- `docker-credentials` - Docker registry credentials
- `kube-config-staging` - Kubernetes config file
- `kube-config-production` - Kubernetes config file

## Best Practices

### 1. Validation First

Always validate before deploying:
```bash
aether -s workload.yaml validate || exit 1
```

### 2. Cost Analysis on PRs

Estimate costs for pull requests:
```yaml
- name: Cost Analysis
  if: github.event_name == 'pull_request'
  run: |
    aether -s workload.yaml cost --provider all
```

### 3. Pre-Deployment Backups

Create backups before production deployments:
```bash
aether backup -n "pre-deploy-$(date +%Y%m%d-%H%M%S)"
```

### 4. Blue-Green Deployments

Use blue-green strategy for zero-downtime:
```bash
aether migrate app kubernetes --strategy blue-green
```

### 5. Automatic Rollback

Implement rollback on failure:
```bash
if ! aether status app; then
  LATEST_BACKUP=$(aether list-backups | head -1 | awk '{print $2}')
  aether restore $LATEST_BACKUP
fi
```

### 6. Health Checks

Verify deployment health:
```bash
kubectl wait --for=condition=ready pod -l app=myapp --timeout=300s
curl -f https://app.example.com/health || exit 1
```

### 7. Resource Cleanup

Clean old resources regularly:
```bash
# Old backups
find ~/.aether/backups -name "*.json" -mtime +7 -delete

# Old images
docker image prune -a --filter "until=720h"
```

## Deployment Strategies

### Immediate Deployment

Fastest, but with downtime:
```bash
aether -s workload.yaml run --runtime kubernetes
```

**Use When:**
- Development environments
- Non-critical services
- Downtime acceptable

### Rolling Deployment

Gradual update with minimal impact:
```bash
aether migrate app kubernetes --strategy rolling
```

**Use When:**
- Production services
- Gradual rollout preferred
- Resource constrained

### Blue-Green Deployment

Zero-downtime with instant rollback:
```bash
aether migrate app kubernetes --strategy blue-green
```

**Use When:**
- Critical production services
- Zero downtime required
- Resources available for parallel deployment

## Environment Management

### Development Environment

```yaml
environment:
  name: development
  auto_deploy: true

scaling:
  minReplicas: 1
  maxReplicas: 2

resources:
  requests:
    cpu: "250m"
    memory: "512Mi"
```

### Staging Environment

```yaml
environment:
  name: staging
  auto_deploy: false  # Manual trigger

scaling:
  minReplicas: 2
  maxReplicas: 5

resources:
  requests:
    cpu: "500m"
    memory: "1Gi"
```

### Production Environment

```yaml
environment:
  name: production
  auto_deploy: false
  require_approval: true

scaling:
  minReplicas: 3
  maxReplicas: 20

resources:
  requests:
    cpu: "1"
    memory: "2Gi"
```

## Security Best Practices

### 1. Secrets Management

**Don't:**
```yaml
env:
  - name: DATABASE_PASSWORD
    value: "plaintext-password"  # ❌ Never do this
```

**Do:**
```yaml
env:
  - name: DATABASE_PASSWORD
    valueFrom:
      secretRef:
        name: db-credentials
        key: password
```

### 2. Image Scanning

Scan images for vulnerabilities:
```bash
# Trivy
trivy image ghcr.io/myorg/app:latest

# Grype
grype ghcr.io/myorg/app:latest
```

### 3. Least Privilege

Use minimal permissions:
```yaml
security:
  runAsNonRoot: true
  runAsUser: 1000
  capabilities:
    drop:
      - ALL
```

### 4. Network Policies

Restrict network access:
```yaml
network:
  policies:
    - name: allow-from-ingress
      podSelector:
        matchLabels:
          app: myapp
      ingress:
        - from:
          - namespaceSelector:
              matchLabels:
                name: ingress-nginx
```

## Monitoring Integration

### Prometheus Metrics

Export metrics during deployment:
```bash
aether metrics > /tmp/metrics.txt
curl -X POST http://pushgateway:9091/metrics/job/deployment < /tmp/metrics.txt
```

### Deployment Tracking

Track deployments in monitoring:
```bash
# Send deployment event to Datadog
curl -X POST "https://api.datadoghq.com/api/v1/events" \
  -H "DD-API-KEY: ${DD_API_KEY}" \
  -d @- << EOF
{
  "title": "Deployment: myapp",
  "text": "Deployed version ${VERSION} to production",
  "tags": ["env:production", "service:myapp"]
}
EOF
```

### Grafana Annotations

Add deployment annotations:
```bash
curl -X POST http://grafana:3000/api/annotations \
  -H "Authorization: Bearer ${GRAFANA_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Deployment: v'"${VERSION}"'",
    "tags": ["deployment", "production"]
  }'
```

## Troubleshooting

### Pipeline Fails at Validation

**Issue:** `Invalid workload specification`

**Solution:**
```bash
# Check YAML syntax
yq eval workload.yaml

# Validate schema
aether -s workload.yaml validate -v
```

### Deployment Timeout

**Issue:** `kubectl wait timeout exceeded`

**Solution:**
```bash
# Check pod status
kubectl get pods -l app=myapp

# Check events
kubectl get events --sort-by='.lastTimestamp'

# Check logs
aether logs myapp
```

### Image Pull Failure

**Issue:** `Failed to pull image`

**Solution:**
```bash
# Verify image exists
docker pull ghcr.io/myorg/app:v1.0.0

# Check image pull secrets
kubectl get secrets

# Create secret if missing
kubectl create secret docker-registry regcred \
  --docker-server=ghcr.io \
  --docker-username=$USER \
  --docker-password=$TOKEN
```

### Rollback Fails

**Issue:** `No backups found`

**Solution:**
```bash
# List available backups
aether list-backups

# Create backup manually
aether backup -n manual-backup

# Restore specific backup
aether restore ~/.aether/backups/backup-20240206.json
```

## Advanced Patterns

### Multi-Region Deployment

```bash
#!/bin/bash
REGIONS=("us-east-1" "us-west-2" "eu-west-1")

for region in "${REGIONS[@]}"; do
  echo "Deploying to $region..."
  export KUBECONFIG=~/.kube/config-$region
  aether -s workload.yaml run --runtime kubernetes
done
```

### Canary Deployment

```bash
# Deploy canary (10% traffic)
yq eval '.scaling.minReplicas = 1' -i workload-canary.yaml
aether -s workload-canary.yaml run

# Monitor metrics
sleep 300

# Check error rate
if [ $(curl -s http://metrics/error_rate) -lt 1 ]; then
  # Promote canary to full deployment
  aether -s workload.yaml run
  aether delete workload-canary
else
  # Rollback canary
  aether delete workload-canary
  exit 1
fi
```

### Feature Flags

```bash
# Deploy with feature flag
yq eval '.env += [{"name": "FEATURE_NEW_UI", "value": "false"}]' -i workload.yaml
aether -s workload.yaml run

# Enable feature
kubectl set env deployment/myapp FEATURE_NEW_UI=true
```

## Performance Optimization

### Parallel Deployments

```yaml
# GitHub Actions
jobs:
  deploy-region-1:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to us-east-1
        run: aether -s workload.yaml run

  deploy-region-2:
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to us-west-2
        run: aether -s workload.yaml run
```

### Caching

```yaml
# Cache Aether binary
- name: Cache Aether
  uses: actions/cache@v3
  with:
    path: /usr/local/bin/aether
    key: aether-${{ env.AETHER_VERSION }}
```

### Incremental Deployments

```bash
# Only deploy if workload spec changed
if git diff --name-only HEAD~1 | grep -q "workload.yaml"; then
  aether -s workload.yaml run
else
  echo "No changes to workload spec, skipping deployment"
fi
```

## Example Repositories

- [aether-demo-app](https://github.com/zyvorai/Aether-demo-app) - Sample application with full CI/CD
- [aether-templates](https://github.com/zyvorai/Aether-templates) - Template repository with pipelines
- [aether-kubernetes](https://github.com/zyvorai/Aether-kubernetes) - Kubernetes-focused examples

## Support

For CI/CD integration help:
- GitHub Issues: https://github.com/zyvorai/Aether/issues
- Tag: `cicd`
- Include: Platform name, pipeline config, error message

## Related Documentation

- [Deployment Guide](DEPLOYMENT.md)
- [Backup Guide](BACKUP.md)
- [Cost Estimation](COST.md)
- [WebUI and API](WEBUI.md)
