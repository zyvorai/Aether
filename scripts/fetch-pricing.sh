#!/usr/bin/env bash
# Fetch cloud pricing and emit JSON suitable for AETHER_PRICING_URL.
# Usage: ./scripts/fetch-pricing.sh > pricing.json
set -euo pipefail

AWS_CPU=15.0
AWS_MEM=4.0
AWS_STORAGE=0.10
AZURE_CPU=14.0
AZURE_MEM=3.5
AZURE_STORAGE=0.12
GCP_CPU=13.0
GCP_MEM=3.5
GCP_STORAGE=0.08

if command -v curl >/dev/null 2>&1 && command -v python3 >/dev/null 2>&1; then
  AWS_JSON="$(curl -sf --max-time 20 \
    'https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonEC2/current/us-east-1/index.json' \
    2>/dev/null || true)"
  if [[ -n "${AWS_JSON:-}" ]]; then
    AWS_CPU="$(python3 -c "
import json, sys
try:
    data = json.loads(sys.argv[1])
    products = data.get('products') or {}
    terms = (data.get('terms') or {}).get('OnDemand') or {}
    target = None
    for sku, prod in products.items():
        attrs = prod.get('attributes') or {}
        if attrs.get('instanceType') != 'm5.large':
            continue
        if attrs.get('operatingSystem', '').lower() != 'linux':
            continue
        if attrs.get('tenancy', '').lower() not in ('shared', ''):
            continue
        if attrs.get('preInstalledSw', 'NA') not in ('NA', ''):
            continue
        target = sku
        break
    if not target:
        sys.exit(0)
    offer = terms.get(target) or {}
    for term in offer.values():
        for dim in term.get('priceDimensions', {}).values():
            hourly = float(dim.get('pricePerUnit', {}).get('USD', 0) or 0)
            if hourly > 0:
                print(round(hourly * 730 / 2, 2))  # m5.large = 2 vCPU
                break
        break
except Exception:
    pass
" "$AWS_JSON" 2>/dev/null || true)"
    if [[ -n "${AWS_CPU:-}" ]] && python3 -c "import sys; sys.exit(0 if float(sys.argv[1])>0 else 1)" "$AWS_CPU" 2>/dev/null; then
      :
    else
      AWS_CPU=15.0
    fi
  fi

  AZURE_JSON="$(curl -sf --max-time 12 \
    'https://prices.azure.com/api/retail/prices?$filter=serviceName%20eq%20%27Virtual%20Machines%27%20and%20armRegionName%20eq%20%27eastus%27&$top=100' \
    2>/dev/null || true)"
  if [[ -n "${AZURE_JSON:-}" ]]; then
    VCPU_HOURLY="$(python3 -c "
import json, sys
try:
    data = json.loads(sys.argv[1])
    for item in data.get('Items') or []:
        sku = (item.get('skuName') or '').lower()
        if 'vcore' in sku or 'vcpu' in sku:
            print(item.get('retailPrice', 0))
            break
except Exception:
    pass
" "$AZURE_JSON" 2>/dev/null || true)"
    if [[ -n "${VCPU_HOURLY:-}" ]] && python3 -c "import sys; sys.exit(0 if float(sys.argv[1])>0 else 1)" "$VCPU_HOURLY" 2>/dev/null; then
      AZURE_CPU="$(python3 -c "print(round(float('$VCPU_HOURLY') * 730, 2))")"
    fi
  fi
fi

cat <<EOF
{
  "aws": {
    "cpu_per_core_monthly": $AWS_CPU,
    "memory_per_gb_monthly": $AWS_MEM,
    "storage_per_gb_monthly": $AWS_STORAGE
  },
  "azure": {
    "cpu_per_core_monthly": $AZURE_CPU,
    "memory_per_gb_monthly": $AZURE_MEM,
    "storage_per_gb_monthly": $AZURE_STORAGE
  },
  "gcp": {
    "cpu_per_core_monthly": $GCP_CPU,
    "memory_per_gb_monthly": $GCP_MEM,
    "storage_per_gb_monthly": $GCP_STORAGE
  },
  "default": {
    "cpu_per_core_monthly": $AWS_CPU,
    "memory_per_gb_monthly": $AWS_MEM,
    "storage_per_gb_monthly": $AWS_STORAGE
  }
}
EOF

echo "Pricing JSON written to stdout (serve via HTTP for AETHER_PRICING_URL)." >&2
