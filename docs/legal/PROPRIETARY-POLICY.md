# Proprietary software policy (draft)

## Position

ZyvorAI Labs does **not** distribute product source or binaries under Apache 2.0, MIT, LGPL, or other open-source licenses, with one approved exception: **Aether-core** (the orchestration engine — CLI, runtime adapters, REST API, web dashboard) is Apache License 2.0. Everything else — **PacketWolf**, **Ragnarok** (including Aether's own confidential-computing features, implemented in `src/ragnarok/` inside the Aether repository but licensed separately from the rest of it), **GuestKit**, **HyperSDK**, and related commercial extensions — remains fully proprietary.

## Third-party dependencies

Compiled products may **link to** third-party open-source libraries (e.g., crates, system libraries). Those components remain governed by their respective licenses. A **NOTICE** file (where provided) lists third-party attributions. That does **not**:

- Grant rights to Zyvor’s proprietary source or binaries
- Permit redistribution of Zyvor products without a written license
- Grant trademark rights

## Customer agreements

Agreements should state:

> All Zyvor product software, documentation, and commercial extensions are proprietary to ZyvorAI Labs Private Limited. Third-party components embedded in builds remain subject only to their own licenses.

## Repositories

- Do not add `LICENSE` files implying Apache/MIT for Zyvor-owned code — **except Aether**, which is the approved exception (Apache-2.0 `LICENSE`, `src/ragnarok/` carved out proprietary).
- Use the company **proprietary LICENSE** (synced via `scripts/sync-proprietary-license.sh`) for every other repo — Aether is deliberately excluded from that script's repo list (see the script's own comment) so it doesn't get overwritten back to a proprietary EULA.
- Keep confidential materials out of public repos; if a repo is private, access is still under proprietary terms unless a separate contract says otherwise. Ragnarok's source (`src/ragnarok/` in the Aether repo) must not be present in the public Aether repository or its history — this requires the confidential-computing trait/plugin extraction described in `src/ragnarok/PROPRIETARY.md` to be complete, and the repo's git history to be scrubbed or restarted, before Aether is actually made public. Until that's done, Aether stays private even though its LICENSE now reads Apache-2.0.

## Contributions

If external contributors are accepted, use a written **CLA** or assignment—counsel to draft. No public OSS release without board approval — Aether's open-sourcing was directed by the founder in-session; confirm that satisfies this requirement or formalize it separately before the repo goes public.
