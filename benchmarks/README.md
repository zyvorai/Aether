# Performance Benchmarking Guide

Aether-specific metrics (deploy, migrate, API) live in [RESULTS.md](RESULTS.md). Run `./benchmarks/aether-bench.sh` for CLI/API baselines.

This guide also covers general load testing with k6 and optimization methodology.

## Overview

This guide covers:
- Performance benchmarking tools and methodologies
- Load testing scenarios
- Performance baselines and targets
- Bottleneck identification
- Optimization techniques
- Continuous performance monitoring

## Benchmarking Tools

### 1. k6 (Load Testing)

**Installation:**
```bash
# macOS
brew install k6

# Linux
wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
tar -xzf k6-v0.48.0-linux-amd64.tar.gz
sudo mv k6 /usr/local/bin/
```

**Basic Load Test:**
```javascript
// load-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '2m', target: 100 },  // Ramp up to 100 users
    { duration: '5m', target: 100 },  // Stay at 100 users
    { duration: '2m', target: 200 },  // Ramp up to 200 users
    { duration: '5m', target: 200 },  // Stay at 200 users
    { duration: '2m', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.01'],
  },
};

export default function () {
  const res = http.get('https://api.example.com/products');

  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });

  sleep(1);
}
```

**Run Test:**
```bash
k6 run load-test.js
```

### 2. Apache Bench (Quick Testing)

```bash
# Install
sudo apt-get install apache2-utils  # Linux
brew install httpd  # macOS

# Simple benchmark
ab -n 10000 -c 100 https://api.example.com/health

# With authentication
ab -n 10000 -c 100 -H "Authorization: Bearer token" https://api.example.com/api/users

# POST requests
ab -n 1000 -c 10 -p data.json -T application/json https://api.example.com/api/orders
```

### 3. Vegeta (Go-based)

```bash
# Install
go install github.com/tsenart/vegeta@latest

# Simple attack
echo "GET https://api.example.com/products" | vegeta attack -duration=30s -rate=100 | vegeta report

# Advanced attack
vegeta attack -duration=60s -rate=500/s -targets=targets.txt | vegeta report -type=text

# Generate report
vegeta attack -duration=60s -rate=500/s -targets=targets.txt | vegeta report -type=json > results.json
vegeta plot results.json > plot.html
```

### 4. wrk (HTTP Benchmarking)

```bash
# Install
git clone https://github.com/wg/wrk.git
cd wrk
make
sudo cp wrk /usr/local/bin/

# Basic benchmark
wrk -t12 -c400 -d30s https://api.example.com/

# With Lua script
wrk -t12 -c400 -d30s -s script.lua https://api.example.com/
```

## Benchmark Scenarios

### Scenario 1: API Endpoint Performance

**Objective:** Measure API response times under load

```javascript
// api-benchmark.js
import http from 'k6/http';
import { check, group, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '1m', target: 50 },
    { duration: '3m', target: 50 },
    { duration: '1m', target: 100 },
    { duration: '3m', target: 100 },
    { duration: '1m', target: 0 },
  ],
};

export default function () {
  group('API Endpoints', () => {
    // Health check
    group('Health', () => {
      const res = http.get('https://api.example.com/health');
      check(res, { 'status 200': (r) => r.status === 200 });
    });

    // List products
    group('Products List', () => {
      const res = http.get('https://api.example.com/api/products');
      check(res, {
        'status 200': (r) => r.status === 200,
        'has products': (r) => JSON.parse(r.body).length > 0,
      });
    });

    // Get product details
    group('Product Detail', () => {
      const res = http.get('https://api.example.com/api/products/1');
      check(res, { 'status 200': (r) => r.status === 200 });
    });

    // Create order
    group('Create Order', () => {
      const payload = JSON.stringify({
        product_id: 1,
        quantity: 2,
      });
      const res = http.post('https://api.example.com/api/orders', payload, {
        headers: { 'Content-Type': 'application/json' },
      });
      check(res, { 'status 201': (r) => r.status === 201 });
    });
  });

  sleep(1);
}
```

### Scenario 2: Database Query Performance

**Objective:** Test database performance under concurrent load

```javascript
// database-benchmark.js
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  scenarios: {
    read_heavy: {
      executor: 'constant-vus',
      vus: 100,
      duration: '5m',
      exec: 'readQueries',
    },
    write_heavy: {
      executor: 'constant-vus',
      vus: 20,
      duration: '5m',
      exec: 'writeQueries',
      startTime: '0s',
    },
  },
};

export function readQueries() {
  // Simulate read-heavy operations
  http.get('https://api.example.com/api/products');
  http.get('https://api.example.com/api/users');
  http.get('https://api.example.com/api/orders');
}

export function writeQueries() {
  // Simulate write operations
  const payload = JSON.stringify({ name: 'Test Product' });
  http.post('https://api.example.com/api/products', payload);
}
```

### Scenario 3: Spike Testing

**Objective:** Test system behavior under sudden traffic spikes

```javascript
// spike-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Normal load
    { duration: '30s', target: 1000 }, // Spike!
    { duration: '2m', target: 100 },   // Recovery
    { duration: '30s', target: 0 },    // Ramp down
  ],
};

export default function () {
  const res = http.get('https://api.example.com/api/products');
  check(res, { 'status 200': (r) => r.status === 200 });
  sleep(1);
}
```

### Scenario 4: Soak Testing

**Objective:** Test system stability over extended period

```javascript
// soak-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '5m', target: 200 },   // Ramp up
    { duration: '4h', target: 200 },   // Stay at load for 4 hours
    { duration: '5m', target: 0 },     // Ramp down
  ],
};

export default function () {
  const res = http.get('https://api.example.com/api/products');
  check(res, { 'status 200': (r) => r.status === 200 });
  sleep(Math.random() * 5); // Random sleep 0-5s
}
```

## Performance Baselines

### Target Metrics

| Metric | Target | Critical Threshold |
|--------|--------|-------------------|
| Response Time (p50) | <100ms | >200ms |
| Response Time (p95) | <500ms | >1000ms |
| Response Time (p99) | <1000ms | >2000ms |
| Throughput | >1000 req/s | <500 req/s |
| Error Rate | <0.1% | >1% |
| CPU Utilization | <70% | >85% |
| Memory Usage | <80% | >90% |
| Database Connections | <80% of max | >95% of max |

### Baseline Establishment

```bash
#!/bin/bash
# establish-baseline.sh

echo "Establishing performance baseline..."

# Run baseline load test
k6 run --out json=baseline-results.json load-test.js

# Extract metrics
cat baseline-results.json | jq -r '
  select(.type=="Point" and .metric=="http_req_duration") |
  .data.value
' | awk '
  BEGIN { sum=0; count=0 }
  { sum+=$1; count++ }
  END { print "Average response time: " sum/count "ms" }
'

# Store baseline
echo "$(date): Baseline established" >> baseline-history.log
cp baseline-results.json "baselines/baseline-$(date +%Y%m%d).json"
```

## Bottleneck Identification

### 1. Application Profiling

**Go (pprof):**
```bash
# Enable profiling endpoint
# In your Go app: import _ "net/http/pprof"

# Capture CPU profile
go tool pprof http://localhost:6060/debug/pprof/profile?seconds=30

# Capture memory profile
go tool pprof http://localhost:6060/debug/pprof/heap

# Analyze goroutines
go tool pprof http://localhost:6060/debug/pprof/goroutine
```

**Node.js:**
```bash
# Install clinic
npm install -g clinic

# CPU profiling
clinic doctor -- node app.js

# Flame graphs
clinic flame -- node app.js

# Bubble profiling
clinic bubbleprof -- node app.js
```

**Python:**
```bash
# Install py-spy
pip install py-spy

# CPU profiling
py-spy record -o profile.svg -- python app.py

# Live top
py-spy top -- python app.py
```

### 2. Database Profiling

**PostgreSQL:**
```sql
-- Enable query logging
ALTER SYSTEM SET log_min_duration_statement = 100;
SELECT pg_reload_conf();

-- Analyze slow queries
SELECT
  query,
  calls,
  total_time,
  mean_time,
  max_time
FROM pg_stat_statements
ORDER BY mean_time DESC
LIMIT 10;

-- Check table bloat
SELECT
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 10;
```

**MySQL:**
```sql
-- Enable slow query log
SET GLOBAL slow_query_log = 'ON';
SET GLOBAL long_query_time = 0.1;

-- Analyze slow queries
SELECT
  DIGEST_TEXT,
  COUNT_STAR,
  AVG_TIMER_WAIT/1000000000000 as avg_time_sec
FROM performance_schema.events_statements_summary_by_digest
ORDER BY AVG_TIMER_WAIT DESC
LIMIT 10;
```

### 3. Network Profiling

```bash
# Install tcpdump
sudo apt-get install tcpdump

# Capture traffic
sudo tcpdump -i any -w capture.pcap port 8080

# Analyze with Wireshark or:
tcpdump -r capture.pcap -n | head -100

# Check connection states
netstat -an | awk '/tcp/ {print $6}' | sort | uniq -c

# Monitor bandwidth
sudo apt-get install iftop
sudo iftop -i eth0
```

### 4. Kubernetes Resource Analysis

```bash
# Top pods by CPU
kubectl top pods --all-namespaces --sort-by=cpu

# Top pods by memory
kubectl top pods --all-namespaces --sort-by=memory

# Check pod resource limits
kubectl get pods -o json | jq -r '
  .items[] |
  "\(.metadata.name): CPU=\(.spec.containers[].resources.limits.cpu // "unlimited") MEM=\(.spec.containers[].resources.limits.memory // "unlimited")"
'

# Analyze resource requests vs usage
kubectl describe nodes | grep -A 5 "Allocated resources"
```

## Performance Optimization

### 1. Application Optimization

**Caching:**
```yaml
# Redis cache deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: redis-cache
spec:
  replicas: 1
  template:
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        command:
        - redis-server
        - --maxmemory 2gb
        - --maxmemory-policy allkeys-lru
        - --save ""  # Disable persistence for pure cache
```

**Connection Pooling:**
```python
# Python example with connection pooling
from sqlalchemy import create_engine
from sqlalchemy.pool import QueuePool

engine = create_engine(
    'postgresql://user:pass@host/db',
    poolclass=QueuePool,
    pool_size=20,
    max_overflow=10,
    pool_pre_ping=True,  # Verify connections
)
```

**Async Processing:**
```javascript
// Node.js async processing with queue
const Queue = require('bull');
const orderQueue = new Queue('orders', 'redis://localhost:6379');

// Process async
orderQueue.process(async (job) => {
  await processOrder(job.data);
});

// Add to queue instead of sync processing
app.post('/orders', async (req, res) => {
  await orderQueue.add(req.body);
  res.status(202).json({ status: 'queued' });
});
```

### 2. Database Optimization

**Indexing:**
```sql
-- Analyze query plan
EXPLAIN ANALYZE SELECT * FROM orders WHERE user_id = 123;

-- Add index
CREATE INDEX CONCURRENTLY idx_orders_user_id ON orders(user_id);

-- Composite index
CREATE INDEX CONCURRENTLY idx_orders_user_status
ON orders(user_id, status);

-- Partial index
CREATE INDEX CONCURRENTLY idx_orders_pending
ON orders(created_at)
WHERE status = 'pending';
```

**Query Optimization:**
```sql
-- Before: N+1 query problem
SELECT * FROM orders;
-- Then for each order:
SELECT * FROM order_items WHERE order_id = ?;

-- After: Join or prefetch
SELECT o.*, oi.*
FROM orders o
LEFT JOIN order_items oi ON oi.order_id = o.id
WHERE o.user_id = 123;
```

**Connection Pooling (PgBouncer):**
```ini
# pgbouncer.ini
[databases]
production = host=postgres port=5432 dbname=production

[pgbouncer]
listen_addr = *
listen_port = 6432
auth_type = md5
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 25
```

### 3. Kubernetes Optimization

**Resource Requests and Limits:**
```yaml
resources:
  requests:
    cpu: "500m"      # Guaranteed CPU
    memory: "1Gi"    # Guaranteed memory
  limits:
    cpu: "2"         # Max CPU (can burst)
    memory: "4Gi"    # Hard memory limit
```

**Horizontal Pod Autoscaling:**
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: api-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: api-server
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
      - type: Percent
        value: 100
        periodSeconds: 30
      - type: Pods
        value: 4
        periodSeconds: 30
      selectPolicy: Max
```

**Pod Disruption Budget:**
```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: api-pdb
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: api-server
```

## Continuous Performance Monitoring

### Prometheus Queries

```promql
# Request rate
rate(http_requests_total[5m])

# Error rate
rate(http_requests_total{status=~"5.."}[5m])
/ rate(http_requests_total[5m])

# Latency percentiles
histogram_quantile(0.95,
  rate(http_request_duration_seconds_bucket[5m])
)

# CPU usage
rate(container_cpu_usage_seconds_total[5m])

# Memory usage
container_memory_working_set_bytes
/ container_spec_memory_limit_bytes * 100
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Performance Metrics",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [{
          "expr": "sum(rate(http_requests_total[5m])) by (service)"
        }]
      },
      {
        "title": "Latency (p95)",
        "targets": [{
          "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le, service))"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m])) * 100"
        }]
      }
    ]
  }
}
```

### Alerting Rules

```yaml
groups:
- name: performance
  rules:
  - alert: HighLatency
    expr: |
      histogram_quantile(0.95,
        rate(http_request_duration_seconds_bucket[5m])
      ) > 1
    for: 5m
    annotations:
      summary: "High latency detected (p95 > 1s)"

  - alert: HighErrorRate
    expr: |
      sum(rate(http_requests_total{status=~"5.."}[5m]))
      / sum(rate(http_requests_total[5m]))
      > 0.01
    for: 5m
    annotations:
      summary: "Error rate above 1%"

  - alert: LowThroughput
    expr: |
      sum(rate(http_requests_total[5m])) < 100
    for: 10m
    annotations:
      summary: "Throughput dropped below 100 req/s"
```

## Performance Testing in CI/CD

### GitHub Actions

```yaml
name: Performance Tests

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install k6
        run: |
          wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
          tar -xzf k6-v0.48.0-linux-amd64.tar.gz
          sudo mv k6 /usr/local/bin/

      - name: Run performance tests
        run: |
          k6 run --out json=results.json benchmarks/load-test.js

      - name: Analyze results
        run: |
          python scripts/analyze-performance.py results.json

      - name: Compare with baseline
        run: |
          python scripts/compare-baseline.py results.json baseline.json

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: performance-results
          path: results.json
```

## Best Practices

1. **Establish Baselines**
   - Run tests in consistent environment
   - Capture metrics before changes
   - Track performance over time

2. **Test Realistic Scenarios**
   - Use production-like data
   - Simulate actual user behavior
   - Include read/write mix

3. **Monitor During Tests**
   - CPU and memory usage
   - Database connections
   - Network I/O
   - Disk I/O

4. **Test Different Load Patterns**
   - Steady load
   - Spike testing
   - Soak testing
   - Stress testing

5. **Optimize Incrementally**
   - Identify bottleneck
   - Make one change
   - Measure impact
   - Repeat

6. **Document Findings**
   - Record baseline metrics
   - Document optimizations
   - Track improvement
   - Share knowledge

7. **Automate Testing**
   - Run regularly in CI/CD
   - Alert on regressions
   - Track trends

## Troubleshooting

### Slow Response Times

1. Check database query performance
2. Verify cache hit rates
3. Analyze API logs for slow endpoints
4. Check network latency
5. Review resource utilization

### High Error Rates

1. Check application logs
2. Verify database connections
3. Check rate limits
4. Review recent deployments
5. Analyze error patterns

### Memory Leaks

1. Profile memory usage over time
2. Check for unclosed connections
3. Review caching strategy
4. Analyze object retention
5. Monitor garbage collection

## Tools Reference

| Tool | Purpose | Best For |
|------|---------|----------|
| k6 | Load testing | API testing, complex scenarios |
| Apache Bench | Quick tests | Simple HTTP benchmarking |
| Vegeta | HTTP load | High throughput testing |
| wrk | HTTP bench | Low-level performance |
| JMeter | Full suite | GUI-based testing |
| Locust | Python-based | Custom scenarios |
| Artillery | Node.js | WebSocket, Socket.IO |

## Additional Resources

- [k6 Documentation](https://k6.io/docs/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Kubernetes Performance Tuning](https://kubernetes.io/docs/concepts/cluster-administration/system-metrics/)
- [Database Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)

## Support

For performance questions:
- GitHub Issues: https://github.com/zyvorai/Aether/issues
- Tag: `performance`
