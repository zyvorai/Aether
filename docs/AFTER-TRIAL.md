# After the Aether Trial

## Get a Commercial Licence

Email **[sales@zyvor.dev](mailto:sales@zyvor.dev)** with subject: `Aether licence request`

## Apply Your Licence Key

```bash
kubectl create secret generic aether-license \
  --from-literal=license.key="<your-key>" \
  -n aether-system

helm upgrade aether oci://ghcr.io/hypersdk/charts/aether \
  --version 0.1.0 \
  --reuse-values \
  --set license.existingSecret="aether-license" \
  -n aether-system

kubectl -n aether-system rollout restart deployment/aether
```

## Verify

```bash
kubectl -n aether-system logs deployment/aether | grep -i "licence"
# Expected: Aether licence: <your-org> — valid until <date>
```
