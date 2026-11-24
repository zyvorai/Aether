# Research: multi-runtime workload specs and portability

Scope: how comparable tools describe one workload for several runtimes, how they
preview and validate changes, and what Aether lacks. Statements marked **[U]**
were not read from a fetched page in this session (written from prior knowledge
and unverified); everything else was read from the cited page.

## What the field does

| Tool | What it does | Read from |
| --- | --- | --- |
| **Score** (CNCF sandbox) | One platform-agnostic `score.yaml` (containers, ports, env, abstract resources such as a database). Implementation CLIs (`score-compose`, `score-k8s`) translate it for one runtime; it deliberately does not say how resources are provisioned. | https://score.dev/ |
| **Podman `kube play`** | Runs Kubernetes YAML on one host. Publishes an explicit per-field support table (supported / unsupported / N/A), and a `--validate` mode (`ignore`, `warn`, `strict`) for unrecognised fields. Unsupported examples: `readinessProbe`, `startupProbe`, `lifecycle.*`, `imagePullSecrets`, `serviceAccountName`, `deployment.strategy`, most `Job` fields; scheduling fields are N/A on a single node. | https://docs.podman.io/en/latest/markdown/podman-kube-play.1.html |
| **kompose** | Converts Compose to Kubernetes. Documents a per-key matrix (converts / not implemented / not applicable) for every Compose key, on top of `compose-go` as the parser. Keys with no Kubernetes equivalent (`depends_on`, `devices`, `ulimits`, `network_mode`, `security_opt`, ...) are marked not applicable. | https://kompose.io/conversion/ |
| **kubeconform** | Validates manifests against the Kubernetes OpenAPI schemas; `-strict` rejects unknown properties and duplicate keys; output as text, json, junit, tap, pretty; exit 0 valid / 1 invalid. No server-side validation. | https://github.com/yannh/kubeconform |
| **Terraform `plan`** | Previews changes without applying (speculative plan). `-detailed-exitcode` returns 0 (no change), 1 (error), 2 (changes) for automation; `-out` saves a plan, which may contain secrets; `-refresh-only` models drift. | https://developer.hashicorp.com/terraform/cli/commands/plan |
| **Open Application Model / KubeVela** | Separates Component, Trait and Policy so one application can bind different runtime capabilities. The docs page fetched was a migration stub, so details are **[U]**. | https://kubevela.io/docs/platform-engineers/oam/oam-model/ (stub) |
| **Radius**, **Humanitec**, **Crossplane compositions** | Abstract resource claims mapped to concrete resources by a platform; Crossplane exposes `crossplane render` to preview a composition offline. **[U]** (the Crossplane docs URL returned 404; Radius/Humanitec not fetched). |
| **kube-linter**, **Polaris**, **Kustomize**, **Helm**, **Compose spec**, **Podman Quadlet**, **KubeVirt VM specs** | Lint/policy checks on manifests, overlays/templating, the Compose file format, systemd-managed containers, VM CRDs. **[U]** (kube-linter's docs page returned only a title). |

## Patterns worth borrowing

1. **Say, per field, what a target will do with it.** Podman and kompose both
   publish a support matrix rather than hoping users discover dropped fields.
   Podman additionally offers `--validate=strict` so unknown fields are an error
   when the user asks for it.
2. **Preview before apply, with a CI-friendly exit code.** Terraform's
   `-detailed-exitcode` (0/1/2) is the established contract.
3. **Plans can leak secrets**; Terraform explicitly warns about it.
4. **Validators report all problems and use stable output formats** (kubeconform:
   text/json/junit, exit code per result).

## What Aether lacked (before this change)

* `aether validate` stops at the first error, gives no line number, and the
  workload structs ignore unknown keys. A typo (`sesionAffinity`) or a
  wrongly-cased key was silently dropped. Writing the linter exposed that
  `network.serviceType`, `networkPolicy` and friends, as documented in the
  schema, the docs and the shipped examples, were never read: the `NetworkSpec`
  struct had no `camelCase` rename. Fixed as part of this work.
* No way to see which spec fields a given runtime ignores. Podman reads about a
  dozen fields; Kubernetes reads nearly all; KubeVirt reads about ten. `compare`
  prints generic prose limits, not field-level facts.
* `aether diff` needs a deployed workload. There was no offline "what would this
  edit do" preview, and nothing telling the user that a change is a rolling
  restart on Kubernetes but a replace on Podman and KubeVirt.
* `serde_yaml` expansion of untrusted specs had no size or alias limits.
* No golden tests for rendered output.

## Ranked ideas

| # | Idea | Rationale | Decision |
| --- | --- | --- | --- |
| 1 | **Spec linter with line-numbered diagnostics** (`aether lint`) | Highest value per line: catches silent typos, hostile input, bad labels; mirrors kubeconform `-strict`. | Built. |
| 2 | **Portability report** (`aether portability`) | Podman/kompose-style field matrix, computed for the user's own spec; also feeds lint rule `W040`. | Built. |
| 3 | **Offline spec plan** (`aether spec-plan`, `--detailed-exit-code`) | Terraform-style preview; adds the per-runtime impact (patch / rolling restart / replace) that no generic tool can give. | Built. |
| 4 | Golden-file tests for rendered manifests | Protects the renderers while the Kubernetes crates move fast (this release moved kube 0.97 to 5). | Built for the Kubernetes Service/Deployment and the KubeVirt VirtualMachine; Podman is argument-list based and covered by existing unit tests. |
| 5 | Rendered-manifest diff in the plan (diff the YAML each adapter would emit) | More precise than field-level impact. | Rejected for now: the adapters build typed k8s-openapi objects behind `pub(crate)` constructors that need a client/image context; a field-level plan driven by the matrix is accurate enough and cheap to keep honest. Revisit once renderers are pure functions. |
| 6 | Import from Score / Compose / `kompose`-style conversion | Large adoption surface. | Rejected: a converter is a product of its own, and `aether compose` already exists. |
| 7 | Crossplane/OAM-style abstract resource claims | Matches Score's resource model. | Rejected: changes the spec's identity (provisioning), out of scope. |
| 8 | Saved plan files (`-out`) | Terraform two-step workflow. | Rejected: saved plans carry secrets; the live `diff` already handles drift. |

## Sources

* https://score.dev/
* https://docs.podman.io/en/latest/markdown/podman-kube-play.1.html
* https://kompose.io/conversion/
* https://github.com/yannh/kubeconform
* https://developer.hashicorp.com/terraform/cli/commands/plan
* https://kubevela.io/docs/platform-engineers/oam/oam-model/ (migration stub only)

## Round 3: output formats, suppression, overlays, canonical render

Pages read in this round (fetched):

* https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/sarif-support-for-code-scanning - SARIF 2.1.0 properties GitHub requires: `version`, `tool.driver.name` and `rules[]`, per result `message.text`, `locations[].physicalLocation.artifactLocation.uri` and a `region` with start/end line and column, and `partialFingerprints` (only `primaryLocationLineHash` is used). Drove `lint --format sarif`.
* https://kubectl.docs.kubernetes.io/references/kustomize/kustomization/patches/ - strategic-merge and JSON 6902 patches applied in order; patches are also called overlays. Drove `spec-merge` (merge patch plus path overrides, offline, on Aether's own spec).
* https://docs.docker.com/reference/cli/docker/compose/config/ - `compose config` prints the resolved canonical model; `--hash` prints a per-service config hash; `--format yaml|json`; `--quiet` validates only. Drove `spec-render` (canonical output, overall and per-section hash).
* https://github.com/yannh/kubeconform (round 2) - junit/tap/json output formats; reused for `--format junit`.

Searched but not confirmed: a web search for kube-linter's check names only
confirmed that `latest-tag`, `no-read-only-root-fs`, `run-as-non-root`,
`unset-cpu-requirements` and `unset-memory-requirements` exist and that checks
can be excluded; the docs page (docs.kubelinter.io) returned only its title, so
the `B1xx` rule set is modelled on those names plus Polaris-style hardening
**[U]**, not copied from a fetched list. The score-k8s documentation URL
returned 404, so Score's override/patch mechanism was not read **[U]**. A search
on drift detection for Nomad/Score returned only vendor blogs and nothing
usable for a design.

Decisions: built SARIF/JUnit/GitHub formats, suppression with a rule catalog,
opt-in best-practice rules, overlays with `--set`, and canonical render with
hashes. Not built: strategic-merge or JSON 6902 patches (merge patch covers
env overlays; list-merge keys need schema knowledge per field), saved plans
(secrets, see round 2), a `--lock-image-digests` equivalent (needs registry
access, outside the offline boundary).

## Round 4: verified rule sources, policy, baselines, manifest validation

Pages read in this round (fetched and read, not recalled):

* https://raw.githubusercontent.com/stackrox/kube-linter/main/docs/generated/checks.md - the full kube-linter check list. Confirms `no-liveness-probe`, `no-readiness-probe`, `run-as-non-root`, `no-read-only-root-fs`, `privilege-escalation-container`, `privileged-container`, `drop-net-raw-capability`, `default-service-account`, `latest-tag`, `minimum-three-replicas`, `no-anti-affinity`, `pdb-max-unavailable`/`pdb-min-available`, `non-isolated-pod`, `host-network`/`host-pid`/`host-ipc`, `docker-sock`, `sensitive-host-mounts`, `env-var-secret`, `exposed-services`, `unset-cpu-requirements`/`unset-memory-requirements`, `schema-validation` (kubeconform). **The B1xx rules are now verified against this list** (mapping in `docs/guides/spec-tools.md`). Checks from the list that Aether does not have yet: privileged container, NET_RAW capability, host namespaces, docker.sock/sensitive host mounts, anti-affinity, missing CPU/memory requests (kube-linter flags them; Aether's `requirements` always has defaults).
* https://raw.githubusercontent.com/FairwindsOps/polaris/master/docs/checks/security.md - Polaris security checks with severities (`runAsRootAllowed` warning, `privilegeEscalationAllowed` danger, `runAsPrivileged` danger, `notReadOnlyRootFilesystem` warning, `hostNetworkSet` warning, `tlsSettingsMissing` warning, `sensitiveContainerEnvVar` danger, ...). Used to cross-check the B1xx set and to choose warning severity for hardening advice.
* https://raw.githubusercontent.com/Score-spec/score-k8s/main/README.md - `score-k8s init` / `generate`, `--override-property path=key` and `--overrides-file` to set or remove values in the Score file before conversion, plus patch templates (a list of `set`/`delete` operations on dot paths). This is the same model as `spec-merge --set path=value` (null deletes), which is therefore consistent with Score; patch templates (Go templates over generated manifests) are not copied.
* https://raw.githubusercontent.com/open-policy-agent/conftest/master/README.md - Conftest runs Rego `deny` rules over structured config and prints `FAIL - file - message` with a pass/warn/fail summary. The page does not document warn semantics or exit codes (**[U]**). Drove `spec-policy`: same idea (rules outside the tool, message per violation, summary line), but a closed declarative operator set so it stays offline and deterministic without a Rego runtime.
* Web search for Kubescape: confirmed control ids `C-0013` (non-root containers) and `C-0017` (immutable container filesystem) and the `kubescape scan framework nsa|mitre` model; framework membership of individual controls was not confirmed **[U]**.
* https://github.com/yannh/kubeconform README (fetched via the raw URL returned 404 this round; the round 2 reading of the repository page stands): `-strict`, `-ignore-missing-schemas`, `-skip`, json/junit/tap output. Drove `manifest-check`.

Not fetched: Datree (project is discontinued **[U]**), Helm lint, kustomize build, Garden and Tilt (dev-loop tools; no spec-validation overlap with this change).

Decisions this round:

| Idea | Decision |
| --- | --- |
| Policy-as-code rule file (`spec-policy`) | Built. Rules evaluated against the spec as written, with line numbers. |
| Baseline drift (`spec-lock write/check`) | Built. Hash-only lock, per-section drift, exit 2 on drift. |
| kubeconform-style manifest validation (`manifest-check`) | Built on the typed `k8s-openapi` objects, not fetched JSON schemas, so it is offline. Required-field detection works by noticing that the typed object serialises defaulted required fields; strictness by diffing keys. Built-in kinds only. |
| `lint --fix` for safe findings | Built for `W020` and `I021` only; `B1xx` advice is opinionated and never auto-fixed. |
| CI / pre-commit snippets (`spec-ci`) | Built. |
| JSON schema generation | Skipped: `schema/workload.schema.json` already exists and is validated in CI by `validate:schema`. |
| Rendered-manifest validation chained into `spec-plan` | Not built; adapters still need a client context to render (see round 2). |
