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
