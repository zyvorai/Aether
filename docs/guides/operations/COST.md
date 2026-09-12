---
hero:
  eyebrow: GUIDES
  title: Cost Estimation Guide
  tone: violet
---

Aether provides cost estimation for workloads across multiple cloud providers, helping you optimize spending and make informed deployment decisions.

## Overview

Cost estimation features:
- **Multi-Provider Comparison**: Compare costs across AWS, Azure, GCP, DigitalOcean, and Linode
- **Resource-Based Pricing**: Accurate estimates based on CPU, memory, and storage
- **Savings Analysis**: Identify the most cost-effective provider
- **Monthly and Hourly Rates**: Flexible cost projections

## Quick Start

### Estimate Costs for All Providers

```bash
aether -s workload.yaml cost
```

Output:
```
💰 Estimating costs...

📊 Cost Estimate for 'myapp'

Resources:
  CPU: 2
  Memory: 4Gi
  Storage: 20Gi

Monthly Cost Estimates:

✅ Linode - $32.00/mo ($0.0438/hr)
  CPU: $10.00/mo | Memory: $20.00/mo | Storage: $2.00/mo
   DigitalOcean - $39.00/mo ($0.0534/hr)
  CPU: $12.00/mo | Memory: $24.00/mo | Storage: $3.00/mo
   Azure - $44.40/mo ($0.0608/hr)
  CPU: $28.00/mo | Memory: $14.00/mo | Storage: $2.40/mo
   GCP - $44.40/mo ($0.0608/hr)
  CPU: $26.00/mo | Memory: $15.00/mo | Storage: $3.40/mo
💰 AWS - $48.00/mo ($0.0658/hr)
  CPU: $30.00/mo | Memory: $16.00/mo | Storage: $2.00/mo

💡 Savings: 33.3% by choosing Linode over AWS
```

### Estimate for Specific Provider

```bash
# AWS
aether -s workload.yaml cost --provider aws

# Azure
aether -s workload.yaml cost -p azure

# GCP
aether -s workload.yaml cost -p gcp

# DigitalOcean
aether -s workload.yaml cost -p digitalocean

# Linode
aether -s workload.yaml cost -p linode
```

### JSON output

```bash
aether --output json -s workload.yaml cost
aether --output json -s workload.yaml cost --provider aws
```

### Live pricing overlay

Publish pricing JSON (from `scripts/fetch-pricing.sh`) and point the API or CLI at it:

```bash
./scripts/fetch-pricing.sh > /tmp/pricing.json
export AETHER_PRICING_URL=http://127.0.0.1:8080/pricing.json   # or file:// when supported
aether -s workload.yaml cost
```

See `GET /api/cost/pricing` for the active source (`baseline` vs `live`) and regional multiplier.

## Pricing Models

### AWS
- **CPU**: $15/vCPU/month
- **Memory**: $4/GB/month
- **Storage**: $0.10/GB/month (EBS gp3)
- **Based on**: t3.medium baseline (us-east-1)

### Azure
- **CPU**: $14/vCPU/month
- **Memory**: $3.50/GB/month
- **Storage**: $0.12/GB/month (Standard SSD)
- **Based on**: B2s baseline (East US)

### GCP
- **CPU**: $13/vCPU/month
- **Memory**: $3.75/GB/month
- **Storage**: $0.17/GB/month (SSD persistent disk)
- **Based on**: e2-medium baseline (us-central1)

### DigitalOcean
- **CPU**: $6/vCPU/month
- **Memory**: $6/GB/month
- **Storage**: $0.15/GB/month (Block storage)
- **Based on**: Basic droplet pricing

### Linode
- **CPU**: $5/vCPU/month
- **Memory**: $5/GB/month
- **Storage**: $0.10/GB/month (Block storage)
- **Based on**: Shared CPU pricing

## Resource Specifications

### CPU Formats

Supported CPU formats:
- **Cores**: `2`, `4`, `8`
- **Millicores**: `1000m` (1 core), `500m` (0.5 cores)

Examples:
```yaml
requirements:
  cpu: "2"      # 2 full cores
  cpu: "1000m"  # 1 core (millicores)
  cpu: "500m"   # 0.5 cores
```

### Memory Formats

Supported memory formats:
- **Gibibytes**: `4Gi`, `8Gi`, `16Gi`
- **Mebibytes**: `2048Mi`, `4096Mi`
- **Gigabytes**: `4GB`, `8GB`
- **Megabytes**: `4000MB`, `8000MB`

Examples:
```yaml
requirements:
  memory: "4Gi"    # 4 GiB
  memory: "2048Mi" # 2 GiB
  memory: "4GB"    # 4 GB
```

### Storage Formats

Same formats as memory (uses same parsing logic).

## Use Cases

### Cost Optimization

Compare providers to find the cheapest option:

```bash
aether -s production-workload.yaml cost | grep "✅"
```

### Budget Planning

Calculate total infrastructure costs:

```bash
#!/bin/bash
# calculate-total-cost.sh

total=0

for workload in workloads/*.yaml; do
  echo "Processing $workload..."
  cost=$(aether -s "$workload" cost -p aws | grep "total_monthly" | awk '{print $2}')
  total=$(echo "$total + $cost" | bc)
done

echo "Total monthly cost: \$${total}"
```

### Provider Selection

Make data-driven provider decisions:

```bash
# Estimate costs for all workloads across all providers
for workload in *.yaml; do
  echo "=== $workload ==="
  aether -s "$workload" cost | grep "Savings"
done
```

### Cost Comparison Report

Generate cost comparison reports:

```bash
aether -s workload.yaml cost > cost-report.txt
cat cost-report.txt
```

## Advanced Usage

### Programmatic Access

Use the Rust API for custom cost analysis:

```rust
use aether::cost::{estimate_all_providers, CloudProvider, CostComparison};
use aether::spec::Workload;

async fn analyze_costs(workload: &Workload) -> anyhow::Result<()> {
    // Get estimates for all providers
    let estimates = estimate_all_providers(workload)?;

    for estimate in estimates {
        println!(
            "{}: ${:.2}/mo",
            estimate.provider,
            estimate.total_monthly
        );
    }

    // Or get cost comparison
    let comparison = CostComparison::for_workload(workload)?;
    println!("Cheapest: {}", comparison.cheapest);
    println!("Savings: {:.1}%", comparison.savings_percentage);

    Ok(())
}
```

### JSON Output

For integration with other tools, parse the output as JSON:

```bash
aether -s workload.yaml cost --format json > cost.json
```

Note: Use `aether --output json cost` for structured `CostComparison` JSON output.

### Cost Trends

Track cost changes over time:

```bash
#!/bin/bash
# cost-tracker.sh

DATE=$(date +%Y-%m-%d)
WORKLOAD="production-app.yaml"

# Estimate current cost
aether -s "$WORKLOAD" cost -p aws | grep "total_monthly" \
  >> "cost-history-${DATE}.log"

# Analyze trends
tail -30 cost-history-*.log
```

## Cost Factors

### Pricing Variables

Actual costs may vary based on:

1. **Region Selection**
   - Different regions have different pricing
   - Cross-region data transfer costs
   - Compliance requirements

2. **Instance Types**
   - Reserved instances (save 30-70%)
   - Spot instances (save up to 90%)
   - Committed use discounts

3. **Volume Discounts**
   - Enterprise agreements
   - Long-term commitments
   - Annual prepayment

4. **Additional Services**
   - Load balancers
   - Network egress
   - Backup storage
   - Monitoring and logging

5. **Support Plans**
   - Basic (free)
   - Developer ($29/mo)
   - Business ($100/mo)
   - Enterprise (custom)

### Hidden Costs

Consider these additional expenses:

- **Networking**: Data transfer, VPN, CDN
- **Storage**: Backups, snapshots, archives
- **Security**: WAF, DDoS protection, encryption
- **Management**: Configuration tools, orchestration
- **Licensing**: OS licenses, software licenses

## Cost Optimization Strategies

### Right-Sizing

Adjust resources to match actual usage:

```yaml
# Over-provisioned (expensive)
requirements:
  cpu: "8"
  memory: "32Gi"
  storage: "500Gi"

# Right-sized (optimized)
requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "50Gi"
```

### Reserved Instances

For stable workloads, use reserved instances:

```
On-Demand: $48/mo
1-Year Reserved: $34/mo (29% savings)
3-Year Reserved: $24/mo (50% savings)
```

### Spot Instances

For fault-tolerant workloads:

```
On-Demand: $48/mo
Spot Instance: $14/mo (71% savings)
```

**Caution:** Spot instances can be terminated anytime.

### Auto-Scaling

Scale down during off-hours:

```yaml
scaling:
  enabled: true
  minReplicas: 1    # Night/weekend
  maxReplicas: 10   # Peak hours
```

**Savings:** ~40% reduction for workloads with predictable traffic patterns.

### Storage Tiering

Use appropriate storage classes:

- **Hot data**: SSD ($0.10-0.17/GB/mo)
- **Warm data**: HDD ($0.04-0.08/GB/mo)
- **Cold data**: Archive ($0.01-0.02/GB/mo)

## Comparison with Actual Billing

### Validation

Compare estimates with actual bills:

```bash
# Estimate
aether -s workload.yaml cost -p aws
# Estimated: $48/mo

# Actual (from AWS bill)
aws ce get-cost-and-usage \
  --time-period Start=2024-01-01,End=2024-02-01 \
  --metrics BlendedCost \
  --granularity MONTHLY
# Actual: $52/mo
```

**Difference:** ~8% variance is common due to:
- Regional pricing differences
- Additional services
- Network egress
- Tax and fees

### Accuracy Improvement

Improve estimate accuracy by:

1. **Region-Specific Pricing**: Update pricing for your region
2. **Instance Type Matching**: Use exact instance type pricing
3. **Reserved Instances**: Account for reserved pricing
4. **Monitoring**: Track actual costs vs. estimates

## Cost Alerts

Set up alerting when costs exceed estimates:

```bash
#!/bin/bash
# cost-alert.sh

WORKLOAD="production-app.yaml"
PROVIDER="aws"
THRESHOLD=100  # $100/mo

# Get estimate
ESTIMATED=$(aether -s "$WORKLOAD" cost -p "$PROVIDER" | \
  grep "total_monthly" | awk '{print $2}' | tr -d '$')

if (( $(echo "$ESTIMATED > $THRESHOLD" | bc -l) )); then
  echo "⚠️  ALERT: Estimated cost \$$ESTIMATED exceeds threshold \$$THRESHOLD"
  # Send notification
  curl -X POST https://alerts.example.com/cost-exceeded \
    -d "workload=$WORKLOAD&cost=$ESTIMATED&threshold=$THRESHOLD"
fi
```

## Multi-Workload Analysis

Analyze costs across all workloads:

```bash
#!/bin/bash
# total-cost-analysis.sh

echo "Cloud Provider Cost Analysis"
echo "============================"

for provider in aws azure gcp digitalocean linode; do
  echo ""
  echo "Provider: $(echo $provider | tr '[:lower:]' '[:upper:]')"
  echo "---"

  total=0

  for workload in workloads/*.yaml; do
    cost=$(aether -s "$workload" cost -p "$provider" 2>/dev/null | \
      grep "total_monthly" | awk '{print $2}' | tr -d '$' || echo "0")

    total=$(echo "$total + $cost" | bc)
  done

  echo "Total: \$${total}/mo"
done
```

## ROI Calculator

Calculate return on investment for migrations:

```bash
# Current costs (on-premise)
CURRENT_COST=5000  # $5000/mo

# Estimated cloud costs
CLOUD_COST=$(aether -s workload.yaml cost -p aws | \
  grep "total_monthly" | awk '{print $2}' | tr -d '$')

# Migration cost
MIGRATION_COST=10000  # $10,000 one-time

# Calculate ROI
MONTHLY_SAVINGS=$(echo "$CURRENT_COST - $CLOUD_COST" | bc)
PAYBACK_MONTHS=$(echo "$MIGRATION_COST / $MONTHLY_SAVINGS" | bc)

echo "Monthly Savings: \$${MONTHLY_SAVINGS}"
echo "Payback Period: ${PAYBACK_MONTHS} months"
```

## Best Practices

1. **Regular Reviews**: Re-estimate costs monthly
2. **Track Actuals**: Compare estimates vs. actual bills
3. **Optimize Continuously**: Right-size based on usage
4. **Use Alerts**: Set up cost threshold alerts
5. **Plan Capacity**: Estimate future growth costs
6. **Document Decisions**: Record provider selection rationale
7. **Benchmark**: Compare across providers regularly

## Limitations

Current limitations:
- Live pricing requires you to host or point `AETHER_PRICING_URL` at a JSON price sheet (no built-in AWS/Azure API client yet)
- Chargeback uses on-disk workload specs; workloads created only via API without persisted YAML are omitted
- Network egress and load balancer costs are included in extended CLI estimates but not in the default chargeback line items

## API & configuration

| Endpoint / env | Purpose |
|----------------|---------|
| `GET /api/cost/pricing` | Active source (`baseline` / `live`), region multiplier, spot/reserved discount % |
| `GET /api/cost/chargeback` | Fleet showback by owner/project with 36-month TCO |
| `POST /api/cost` | Per-provider estimates for a workload spec |
| `AETHER_COST_PROVIDER` | Default provider (`aws`, `azure`, `gcp`, …) |
| `AETHER_COST_REGION` | Regional multiplier (e.g. `eu-west-1`) |
| `AETHER_PRICING_URL` | Optional HTTP JSON overlay (cached 1h) |
| `AETHER_COST_SPOT_DISCOUNT_PCT` | Spot/preemptible discount (default 60) |
| `AETHER_COST_RESERVED_DISCOUNT_PCT` | Reserved discount (default 35) |

Alert rule `CostExceeds` compares fleet and per-workload estimates from this pricing path during reconciliation and API health loops.

## Support

For cost estimation issues:
- GitHub Issues: https://github.com/zyvorai/Aether/issues
- Tag: `cost-estimation`

## Additional Resources

- [AWS Pricing Calculator](https://calculator.aws/)
- [Azure Pricing Calculator](https://azure.microsoft.com/pricing/calculator/)
- [GCP Pricing Calculator](https://cloud.google.com/products/calculator)
- [DigitalOcean Pricing](https://www.digitalocean.com/pricing)
- [Linode Pricing](https://www.linode.com/pricing/)
