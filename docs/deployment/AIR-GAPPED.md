# Air-Gapped Deployment

> Install and operate Aether without internet access.

---

## Build offline bundle

On a connected build host:

```bash
./scripts/package-binary-remote.sh
# Produces tarball with binary, docs, welcome PDF, cluster scripts
```

Transfer tarball to air-gapped environment via approved media.

---

## Install

```bash
tar xzf aether-customer-*.tar.gz
cd aether-*/
cat OPEN_FIRST.txt
./install.sh   # if provided in bundle
```

Verify:

```bash
./bin/aether --version
./bin/aether validate --spec examples/workload.yaml
```

---

## Offline images

1. Mirror container base images to internal registry
2. Update workload `build.registry` to internal mirror
3. Pre-load Podman images on dev hosts

---

## Dashboard

Serve embedded static assets from `aether serve` — no CDN required.

---

## Updates

Ship new tarball each release; do not pull from public registries in gap.

---

## Security

- Generate `AETHER_SECRET_KEY` and `AETHER_API_KEY` inside gap
- Issue RBAC keys per operator
- See [ENTERPRISE-SECURITY.md](../guides/security/ENTERPRISE-SECURITY.md)

---

## Related

- `scripts/lib/START_HERE.txt` — customer onboarding
- [Production Reference](PRODUCTION-REFERENCE.md)
