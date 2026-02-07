# Production Runbook

Operational procedures, incident response, and troubleshooting guide for Orchestr8 deployments.

## Table of Contents

1. [Overview](#overview)
2. [Pre-Deployment Checklist](#pre-deployment-checklist)
3. [Deployment Procedures](#deployment-procedures)
4. [Monitoring & Alerts](#monitoring--alerts)
5. [Incident Response](#incident-response)
6. [Common Issues](#common-issues)
7. [Rollback Procedures](#rollback-procedures)
8. [Scaling Operations](#scaling-operations)
9. [Backup & Recovery](#backup--recovery)
10. [Security Incidents](#security-incidents)
11. [Performance Tuning](#performance-tuning)
12. [Maintenance Windows](#maintenance-windows)

## Overview

This runbook provides step-by-step procedures for managing Orchestr8 deployments in production environments.

### On-Call Contact Information

```
Primary On-Call: Team rotation (see PagerDuty)
Escalation: Platform Team Lead
Emergency: CTO

Slack Channels:
- #orchestr8-alerts
- #orchestr8-incidents
- #platform-team
```

### Critical Services

| Service | Priority | RTO | RPO | Contact |
|---------|----------|-----|-----|---------|
| API Gateway | P0 | 5min | 0 | Platform Team |
| Payment Service | P0 | 5min | 0 | Payments Team |
| User Service | P1 | 15min | 1min | Identity Team |
| Product Service | P1 | 15min | 5min | Product Team |
| Database | P0 | 10min | 1min | Data Team |

## Pre-Deployment Checklist

### Before Any Deployment

- [ ] Create backup: `orchestr8 backup -n pre-deploy-$(date +%Y%m%d)`
- [ ] Verify health checks pass
- [ ] Review recent alerts and metrics
- [ ] Check resource capacity
- [ ] Notify stakeholders in #deployments
- [ ] Ensure rollback plan is ready
- [ ] Schedule deployment during low-traffic window
- [ ] Have second team member available

### Production Deployment Checklist

- [ ] All tests pass in staging
- [ ] Load testing completed
- [ ] Security scan completed (no HIGH/CRITICAL)
- [ ] Database migrations tested
- [ ] Feature flags configured
- [ ] Runbook updated with new procedures
- [ ] Monitoring dashboards updated
- [ ] Rollback tested in staging
- [ ] Change request approved
- [ ] Customer support team notified

## Deployment Procedures

### Standard Deployment

```bash
#!/bin/bash
# standard-deployment.sh

set -e

echo "Starting deployment of $SERVICE_NAME v$VERSION"

# 1. Pre-deployment checks
echo "Running pre-deployment checks..."
orchestr8 -s workloads/$SERVICE_NAME.yaml validate
kubectl get nodes
kubectl get pods -n production

# 2. Create backup
echo "Creating backup..."
BACKUP_NAME="pre-deploy-$SERVICE_NAME-$(date +%Y%m%d-%H%M%S)"
orchestr8 backup -n "$BACKUP_NAME" -d "Pre-deployment backup for $SERVICE_NAME v$VERSION"

# 3. Deploy with blue-green strategy
echo "Deploying $SERVICE_NAME v$VERSION..."
orchestr8 migrate $SERVICE_NAME kubernetes --strategy blue-green

# 4. Wait for deployment
echo "Waiting for deployment to stabilize..."
kubectl wait --for=condition=ready pod -l app=$SERVICE_NAME --timeout=300s

# 5. Run smoke tests
echo "Running smoke tests..."
./scripts/smoke-test-$SERVICE_NAME.sh

# 6. Verify health
echo "Checking service health..."
HEALTH_URL=$(kubectl get svc $SERVICE_NAME -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')
curl -f https://$HEALTH_URL/health || {
    echo "Health check failed! Initiating rollback..."
    orchestr8 restore $(orchestr8 list-backups | grep $BACKUP_NAME | awk '{print $2}')
    exit 1
}

# 7. Monitor for 5 minutes
echo "Monitoring deployment..."
for i in {1..30}; do
    ERROR_RATE=$(curl -s http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~"5.."}[1m]) | jq -r '.data.result[0].value[1]')
    if (( $(echo "$ERROR_RATE > 0.01" | bc -l) )); then
        echo "Error rate too high! Rolling back..."
        orchestr8 restore $(orchestr8 list-backups | grep $BACKUP_NAME | awk '{print $2}')
        exit 1
    fi
    sleep 10
done

echo "✅ Deployment successful!"
echo "Backup available: $BACKUP_NAME"
```

### Emergency Hotfix Deployment

```bash
#!/bin/bash
# emergency-hotfix.sh

# Skip normal approval process for P0 incidents
# Still maintain safety checks

echo "🚨 EMERGENCY HOTFIX DEPLOYMENT"

# Quick validation
orchestr8 -s workloads/$SERVICE_NAME.yaml validate

# Deploy immediately
orchestr8 migrate $SERVICE_NAME kubernetes --strategy immediate

# Monitor closely
watch -n 5 kubectl get pods -l app=$SERVICE_NAME
```

### Canary Deployment

```bash
#!/bin/bash
# canary-deployment.sh

# Deploy to 10% of traffic
yq eval '.scaling.minReplicas = 1' -i workloads/$SERVICE_NAME-canary.yaml
orchestr8 -s workloads/$SERVICE_NAME-canary.yaml run

# Monitor for 15 minutes
sleep 900

# Check error rate
ERROR_RATE=$(get-error-rate.sh $SERVICE_NAME-canary)
if [ $(echo "$ERROR_RATE < 0.01" | bc) -eq 1 ]; then
    # Promote canary
    orchestr8 -s workloads/$SERVICE_NAME.yaml run
    orchestr8 delete $SERVICE_NAME-canary
else
    # Rollback canary
    orchestr8 delete $SERVICE_NAME-canary
    echo "Canary failed - not promoting"
fi
```

## Monitoring & Alerts

### Key Metrics to Monitor

```yaml
# Critical Metrics
- name: service_availability
  threshold: < 99.9%
  alert: P0

- name: error_rate
  threshold: > 1%
  alert: P1

- name: latency_p95
  threshold: > 500ms
  alert: P1

- name: latency_p99
  threshold: > 1000ms
  alert: P2

- name: cpu_utilization
  threshold: > 80%
  alert: P2

- name: memory_utilization
  threshold: > 85%
  alert: P2

- name: disk_utilization
  threshold: > 90%
  alert: P1
```

### Dashboard URLs

```
Production Overview:   https://grafana.company.com/d/prod-overview
Service Health:        https://grafana.company.com/d/service-health
Infrastructure:        https://grafana.company.com/d/infrastructure
Database Performance:  https://grafana.company.com/d/database
```

### Alert Response Times

| Severity | Response Time | Resolution Time |
|----------|--------------|-----------------|
| P0 | 5 minutes | 1 hour |
| P1 | 15 minutes | 4 hours |
| P2 | 1 hour | 24 hours |
| P3 | 4 hours | 1 week |

## Incident Response

### Incident Response Workflow

```
1. DETECT
   ├─ Alert fires
   ├─ Page on-call engineer
   └─ Create incident ticket

2. ASSESS
   ├─ Check monitoring dashboards
   ├─ Review recent deployments
   ├─ Check logs for errors
   └─ Determine severity

3. MITIGATE
   ├─ Apply immediate fix or rollback
   ├─ Update incident status
   └─ Communicate with stakeholders

4. RESOLVE
   ├─ Verify fix is effective
   ├─ Monitor for recurrence
   └─ Close incident

5. FOLLOW-UP
   ├─ Write post-mortem
   ├─ Implement preventive measures
   └─ Update runbook
```

### P0 Incident Procedure

```bash
# STEP 1: Acknowledge incident
# - Page acknowledged in PagerDuty
# - Post in #orchestr8-incidents
# - Start war room if needed

# STEP 2: Quick assessment
kubectl get pods -n production
kubectl get events --sort-by='.lastTimestamp' | tail -20
orchestr8 list

# STEP 3: Check recent changes
git log --since="1 hour ago" --oneline
kubectl rollout history deployment/$SERVICE_NAME

# STEP 4: Review metrics
# Open Grafana dashboards
# Check error rates, latency, throughput

# STEP 5: Immediate mitigation
# Option A: Rollback recent deployment
LAST_BACKUP=$(orchestr8 list-backups | head -2 | tail -1 | awk '{print $2}')
orchestr8 restore $LAST_BACKUP

# Option B: Scale up resources
kubectl scale deployment/$SERVICE_NAME --replicas=10

# Option C: Route traffic away
kubectl patch ingress $SERVICE_NAME -p '{"spec":{"rules":[]}}'

# STEP 6: Communicate
# Post updates every 15 minutes in #orchestr8-incidents
# Update status page if customer-facing

# STEP 7: Verify resolution
curl -f https://api.company.com/health
# Check error rates return to normal

# STEP 8: Schedule post-mortem
# Create calendar invite within 24 hours
```

## Common Issues

### High Error Rate

**Symptoms:**
- HTTP 5xx errors increasing
- Error rate alerts firing
- Customer complaints

**Diagnosis:**
```bash
# Check error logs
kubectl logs -n production -l app=$SERVICE_NAME --tail=100 | grep ERROR

# Check recent deployments
kubectl rollout history deployment/$SERVICE_NAME

# Check resource usage
kubectl top pods -n production -l app=$SERVICE_NAME

# Check dependencies
curl -f https://database:5432/health
curl -f https://redis:6379/health
```

**Resolution:**
```bash
# If recent deployment:
orchestr8 restore $(orchestr8 list-backups | head -2 | tail -1 | awk '{print $2}')

# If resource exhaustion:
kubectl scale deployment/$SERVICE_NAME --replicas=$(($(kubectl get deployment $SERVICE_NAME -o jsonpath='{.spec.replicas}') * 2))

# If dependency issue:
# Fix dependency first, then restart service
kubectl rollout restart deployment/$SERVICE_NAME
```

### High Latency

**Symptoms:**
- Response times >1s
- Latency alerts firing
- Slow page loads

**Diagnosis:**
```bash
# Check service latency
kubectl exec -it $POD_NAME -- curl http://localhost:8080/metrics | grep latency

# Check database performance
kubectl exec -it postgres-0 -- psql -U postgres -c "
  SELECT pid, now() - query_start AS duration, query
  FROM pg_stat_activity
  WHERE state = 'active'
  ORDER BY duration DESC
  LIMIT 10;"

# Check cache hit rate
kubectl exec -it redis-0 -- redis-cli INFO stats | grep hits
```

**Resolution:**
```bash
# Enable caching
kubectl set env deployment/$SERVICE_NAME CACHE_ENABLED=true

# Scale up
kubectl scale deployment/$SERVICE_NAME --replicas=10

# Optimize database queries
# (requires code changes)

# Add read replicas
orchestr8 -s workloads/database-replica.yaml run
```

### Out of Memory (OOM)

**Symptoms:**
- Pods restarting frequently
- OOMKilled events
- Memory alerts

**Diagnosis:**
```bash
# Check OOM events
kubectl get events -n production | grep OOMKilled

# Check memory usage
kubectl top pods -n production -l app=$SERVICE_NAME

# Check memory limits
kubectl get deployment $SERVICE_NAME -o yaml | grep -A 5 resources
```

**Resolution:**
```bash
# Increase memory limits
yq eval '.resources.limits.memory = "4Gi"' -i workloads/$SERVICE_NAME.yaml
orchestr8 -s workloads/$SERVICE_NAME.yaml run

# Or fix memory leak
# (requires code changes and redeployment)

# Temporary: Restart pods regularly
kubectl set env deployment/$SERVICE_NAME RESTART_INTERVAL=3600
```

### Database Connection Pool Exhausted

**Symptoms:**
- "Too many connections" errors
- Database alerts
- Slow queries

**Diagnosis:**
```bash
# Check active connections
kubectl exec -it postgres-0 -- psql -U postgres -c "
  SELECT count(*), state
  FROM pg_stat_activity
  GROUP BY state;"

# Check max connections
kubectl exec -it postgres-0 -- psql -U postgres -c "
  SHOW max_connections;"
```

**Resolution:**
```bash
# Increase max connections
kubectl exec -it postgres-0 -- psql -U postgres -c "
  ALTER SYSTEM SET max_connections = 200;
  SELECT pg_reload_conf();"

# Optimize connection pooling
kubectl set env deployment/$SERVICE_NAME DB_POOL_SIZE=20 DB_POOL_MAX_OVERFLOW=10

# Add connection pooler (PgBouncer)
orchestr8 -s workloads/pgbouncer.yaml run
```

### Certificate Expiration

**Symptoms:**
- SSL/TLS errors
- Certificate warnings
- Connection refused

**Diagnosis:**
```bash
# Check certificate expiration
kubectl get certificate -n production
kubectl describe certificate $CERT_NAME

# Manual check
echo | openssl s_client -servername api.company.com -connect api.company.com:443 2>/dev/null | openssl x509 -noout -dates
```

**Resolution:**
```bash
# Renew with cert-manager
kubectl delete certificate $CERT_NAME
kubectl apply -f certificates/$CERT_NAME.yaml

# Or manual renewal
# (depends on certificate provider)

# Verify renewal
kubectl get certificate $CERT_NAME -o yaml | grep notAfter
```

## Rollback Procedures

### Standard Rollback

```bash
#!/bin/bash
# rollback.sh

set -e

echo "🔄 Starting rollback of $SERVICE_NAME"

# 1. Identify backup to restore
orchestr8 list-backups
read -p "Enter backup name to restore: " BACKUP_NAME

# 2. Verify backup
orchestr8 list-backups | grep $BACKUP_NAME || {
    echo "Backup not found!"
    exit 1
}

# 3. Announce rollback
echo "Announcing rollback in #orchestr8-incidents..."
# Post to Slack

# 4. Execute rollback
echo "Restoring from backup: $BACKUP_NAME"
orchestr8 restore ~/.orchestr8/backups/$BACKUP_NAME.json

# 5. Verify rollback
echo "Verifying rollback..."
kubectl wait --for=condition=ready pod -l app=$SERVICE_NAME --timeout=300s

# 6. Run health checks
echo "Running health checks..."
./scripts/smoke-test-$SERVICE_NAME.sh

# 7. Monitor
echo "Monitoring for 5 minutes..."
sleep 300

# 8. Confirm success
echo "✅ Rollback completed successfully"
echo "Previous version restored from $BACKUP_NAME"
```

### Emergency Rollback

```bash
# Fastest possible rollback
LAST_BACKUP=$(orchestr8 list-backups | head -2 | tail -1 | awk '{print $2}')
orchestr8 restore $LAST_BACKUP
```

### Database Rollback

```bash
#!/bin/bash
# database-rollback.sh

# Stop application writes
kubectl scale deployment/$SERVICE_NAME --replicas=0

# Restore database backup
kubectl exec -it postgres-0 -- pg_restore \
  -U postgres \
  -d production \
  -c \
  /backups/database-$(date -d yesterday +%Y%m%d).dump

# Restart application
kubectl scale deployment/$SERVICE_NAME --replicas=3
```

## Scaling Operations

### Manual Scaling

```bash
# Scale up
kubectl scale deployment/$SERVICE_NAME --replicas=10

# Scale down (during low traffic)
kubectl scale deployment/$SERVICE_NAME --replicas=3

# Check scaling status
kubectl get hpa
kubectl top pods -n production
```

### Auto-Scaling Tuning

```bash
# Adjust HPA targets
kubectl autoscale deployment/$SERVICE_NAME \
  --cpu-percent=70 \
  --min=3 \
  --max=20

# Check HPA status
kubectl describe hpa $SERVICE_NAME

# View scaling events
kubectl get events | grep HorizontalPodAutoscaler
```

### Database Scaling

```bash
# Add read replica
orchestr8 -s workloads/database-replica.yaml run

# Update application to use replica for reads
kubectl set env deployment/$SERVICE_NAME \
  READ_REPLICA_HOST=postgres-replica.production.svc.cluster.local

# Verify replica lag
kubectl exec -it postgres-replica-0 -- psql -U postgres -c "
  SELECT now() - pg_last_xact_replay_timestamp() AS replication_lag;"
```

## Backup & Recovery

### Daily Backup Procedure

```bash
#!/bin/bash
# daily-backup.sh

DATE=$(date +%Y%m%d)

# Orchestr8 state backup
orchestr8 backup -n "daily-$DATE" -d "Automated daily backup"

# Database backup
kubectl exec -it postgres-0 -- pg_dump \
  -U postgres \
  -Fc \
  production > /backups/database-$DATE.dump

# Upload to S3
aws s3 cp /backups/database-$DATE.dump \
  s3://company-backups/orchestr8/database-$DATE.dump

# Cleanup old backups (keep 30 days)
find /backups -name "*.dump" -mtime +30 -delete
orchestr8 list-backups | tail -n +32 | awk '{print $2}' | xargs -I {} rm {}
```

### Disaster Recovery Test

```bash
#!/bin/bash
# dr-test.sh

# Monthly DR test procedure

# 1. Spin up DR environment
kubectl create namespace dr-test

# 2. Restore from backup
LATEST_BACKUP=$(orchestr8 list-backups | head -1 | awk '{print $2}')
orchestr8 restore $LATEST_BACKUP --namespace dr-test

# 3. Restore database
kubectl exec -it postgres-0 -n dr-test -- pg_restore \
  -U postgres \
  -d production \
  /backups/latest.dump

# 4. Run smoke tests
./scripts/smoke-test-dr.sh

# 5. Document results
echo "DR test completed: $(date)" >> dr-test-log.txt

# 6. Cleanup
kubectl delete namespace dr-test
```

## Security Incidents

### Suspected Breach Procedure

```bash
# 1. IMMEDIATE ACTIONS
# - Isolate affected systems
kubectl cordon $AFFECTED_NODE
kubectl drain $AFFECTED_NODE --ignore-daemonsets

# - Revoke credentials
kubectl delete secret $COMPROMISED_SECRET
# Rotate all API keys and passwords

# - Enable audit logging
kubectl apply -f audit-policy.yaml

# 2. INVESTIGATION
# - Collect logs
kubectl logs -n production --all-containers=true --since=24h > incident-logs.txt

# - Check for suspicious activity
grep "unauthorized" incident-logs.txt
grep "failed login" incident-logs.txt

# - Review access logs
kubectl get events --sort-by='.lastTimestamp' | tail -100

# 3. CONTAINMENT
# - Block suspicious IPs
kubectl apply -f network-policy-lockdown.yaml

# - Disable compromised accounts
kubectl delete serviceaccount $COMPROMISED_SA

# 4. RECOVERY
# - Deploy patched versions
orchestr8 -s workloads-patched/$SERVICE_NAME.yaml run

# - Verify integrity
./scripts/verify-checksums.sh

# 5. POST-INCIDENT
# - Write incident report
# - Update security policies
# - Schedule security audit
```

### CVE Response

```bash
#!/bin/bash
# cve-response.sh

CVE_ID=$1
SEVERITY=$2

echo "Responding to $CVE_ID (Severity: $SEVERITY)"

# 1. Assess impact
trivy image --severity $SEVERITY ghcr.io/company/app:latest

# 2. Update base images
# (requires Dockerfile changes)

# 3. Rebuild images
docker build -t ghcr.io/company/app:patched .
docker push ghcr.io/company/app:patched

# 4. Deploy updates
yq eval '.image.tag = "patched"' -i workloads/*.yaml
for service in workloads/*.yaml; do
    orchestr8 -s "$service" run
done

# 5. Verify patches
trivy image ghcr.io/company/app:patched
```

## Performance Tuning

### Database Performance

```bash
# Analyze slow queries
kubectl exec -it postgres-0 -- psql -U postgres -c "
  SELECT query, calls, total_time, mean_time
  FROM pg_stat_statements
  ORDER BY mean_time DESC
  LIMIT 10;"

# Add indexes
kubectl exec -it postgres-0 -- psql -U postgres -c "
  CREATE INDEX CONCURRENTLY idx_users_email ON users(email);"

# Vacuum and analyze
kubectl exec -it postgres-0 -- psql -U postgres -c "
  VACUUM ANALYZE;"
```

### Cache Optimization

```bash
# Check cache hit rate
CACHE_HIT_RATE=$(kubectl exec -it redis-0 -- redis-cli INFO stats | grep keyspace_hits | awk -F: '{print $2}')
echo "Cache hit rate: $CACHE_HIT_RATE%"

# Adjust cache size
kubectl set env deployment/redis MAXMEMORY=16gb

# Tune eviction policy
kubectl exec -it redis-0 -- redis-cli CONFIG SET maxmemory-policy allkeys-lru
```

### Application Optimization

```bash
# Enable HTTP/2
kubectl annotate ingress $SERVICE_NAME \
  nginx.ingress.kubernetes.io/http2-push-preload=true

# Enable compression
kubectl annotate ingress $SERVICE_NAME \
  nginx.ingress.kubernetes.io/enable-compression=true

# Adjust worker processes
kubectl set env deployment/$SERVICE_NAME WORKERS=4
```

## Maintenance Windows

### Planned Maintenance Procedure

```bash
#!/bin/bash
# planned-maintenance.sh

# Typically scheduled for Sunday 2-4 AM

# 1. Pre-maintenance
echo "Starting pre-maintenance checks..."
orchestr8 backup -n "pre-maintenance-$(date +%Y%m%d)"
./scripts/smoke-test-all.sh

# 2. Announce maintenance
# Post in #announcements and update status page

# 3. Scale up for redundancy
kubectl scale deployment/api-gateway --replicas=5

# 4. Perform maintenance
# - Apply updates
# - Run migrations
# - Update configurations

# 5. Verify functionality
./scripts/smoke-test-all.sh

# 6. Monitor for issues
sleep 600  # 10 minutes

# 7. Scale back to normal
kubectl scale deployment/api-gateway --replicas=3

# 8. Complete maintenance
# Update status page
echo "Maintenance completed: $(date)"
```

### Database Maintenance

```bash
# Vacuum full (requires downtime)
kubectl scale deployment/$SERVICE_NAME --replicas=0
kubectl exec -it postgres-0 -- psql -U postgres -c "VACUUM FULL;"
kubectl scale deployment/$SERVICE_NAME --replicas=3

# Reindex
kubectl exec -it postgres-0 -- psql -U postgres -c "REINDEX DATABASE production;"

# Update statistics
kubectl exec -it postgres-0 -- psql -U postgres -c "ANALYZE;"
```

## Appendix

### Useful Commands

```bash
# Quick health check all services
kubectl get pods -n production -o wide

# Check recent events
kubectl get events --sort-by='.lastTimestamp' | tail -20

# View resource usage
kubectl top nodes
kubectl top pods -n production

# Check ingress status
kubectl get ingress -n production

# View logs from all pods
kubectl logs -n production -l app=$SERVICE_NAME --all-containers=true

# Execute command in pod
kubectl exec -it $POD_NAME -- /bin/bash

# Port forward for debugging
kubectl port-forward svc/$SERVICE_NAME 8080:8080

# Copy files from pod
kubectl cp $POD_NAME:/path/to/file ./local-file
```

### Contact Information

```
Platform Team: platform-team@company.com
Database Team: database-team@company.com
Security Team: security-team@company.com
DevOps Lead: devops-lead@company.com

Vendor Support:
- Cloud Provider: 1-800-XXX-XXXX
- Database: support@database.com
- Monitoring: support@monitoring.com
```

### Change Log

| Date | Author | Changes |
|------|--------|---------|
| 2024-02-06 | Platform Team | Initial runbook creation |
| | | Added incident response procedures |
| | | Added rollback procedures |

## Support

For runbook questions or updates:
- GitHub Issues: https://github.com/ssahani/orchestr8/issues
- Tag: `runbook`
