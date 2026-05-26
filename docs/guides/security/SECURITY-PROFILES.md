# Security profiles (Aether + Ragnarok)

Use **security profiles** in workload specs instead of hard-coding `runtimeClassName: kata-clh-snp`.

```yaml
confidential:
  enabled: true
  securityProfile: sovereign-high
  tee: sev-snp
```

When `RAGNAROK_URL` is set, Aether resolves the profile against Ragnarok's runtime pool catalog (`GET /api/v1/confidential/security-profiles`).

See [Ragnarok SECURITY-PROFILES.md](https://github.com/ssahani/ragnarok/blob/main/docs/SECURITY-PROFILES.md) for pool CRDs and scheduler admission.
