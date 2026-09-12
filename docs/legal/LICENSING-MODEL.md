# Licensing model (draft)

**Aether-core is the one exception to an otherwise all-proprietary lineup.** Since 2026 the Aether orchestration engine (CLI, runtime adapters, REST API, web dashboard) is Apache License 2.0, open source — see [LICENSE](https://github.com/zyvorai/Aether/blob/main/LICENSE). Everything else — PacketWolf, Ragnarok (confidential computing; separate proprietary repository), HyperSDK, and GuestKit — remains proprietary with no open-source (Apache, MIT, LGPL, or similar) distribution. Access to the proprietary products is by written agreement or the applicable EULA.

## License types

| Layer | License | Products |
|-------|---------|----------|
| Open source | Apache License 2.0 | Aether-core |
| Self-hosted / binaries | Proprietary EULA | PacketWolf, Ragnarok, GuestKit, HyperSDK tooling |
| Enterprise subscription | MSA + ELA + Order Form | Full feature set per tier |
| Hosted SaaS (if offered) | Proprietary + MSA | zyvor.dev cloud |
| Branding | Trademark policy | All product names, incl. "Aether"/"Zyvor" — the Apache 2.0 grant does not include trademark rights |
| AI models / rules / automation packs | Commercial | NetPredator intelligence, remediation |

Third-party libraries used in builds (e.g., Rust crates) remain subject to **their** licenses; that does not make Zyvor’s product source or binaries open source.

## Tiers (suggested)

| Tier | Audience | Rights |
|------|----------|--------|
| Evaluation | Qualified prospects | Time-limited proprietary license, no production |
| Professional | SMB | Production use, standard support |
| Enterprise | Regulated / large IT | Full features, SLA option |
| Sovereign | Government / critical infra | Sovereign features + compliance addenda |
| Hyperscale | Cloud / MSP | Volume Order Form, custom DPA |

## Commercial metrics (Order Form)

License by transparent metrics—avoid surprise audits:

| Metric | Notes |
|--------|--------|
| Production clusters | Per K8s cluster or control plane |
| CPU sockets / cores | Optional cap |
| Confidential / TEE nodes | Premium |
| Tenant trust domains | Premium |
| GPU confidential pools | Premium |
| Managed workloads / nodes observed | PacketWolf-style metering |
| Named support contacts | SLA tiering |
| Term | Annual default |

## Feature gates (examples)

| Capability | Standard | Enterprise |
|------------|----------|------------|
| Basic orchestration / dashboards | Yes | Yes |
| Multi-tenant trust domains | No | Yes |
| Sovereign / air-gap deployment packs | No | Yes |
| Attestation management UI | No | Yes |
| Runtime policy enforcement (eBPF/TC) | Per Order Form | Yes |
| Fleet / multi-cluster sync | No | Yes |
| AI remediation / auto-policy apply | No | Yes |
| Enterprise audit export / SIEM bundle | No | Yes |
| Cross-region trust federation | No | Yes |

Adjust per product—see [PRODUCT-MATRIX.md](PRODUCT-MATRIX.md).
