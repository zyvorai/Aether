---
hero:
  eyebrow: GUIDES
  title: Spec tools - lint, portability, plan, merge, render
  tone: violet
  lead: "Check a spec, see what each runtime will ignore, and preview an edit without applying it."
---

Three offline commands work on spec files only. They never contact Podman,
Kubernetes or KubeVirt and need no running workload.

| Command | Question it answers |
| --- | --- |
| `aether lint` | What is wrong with this spec, and on which line? |
| `aether portability` | Which of my fields will each runtime silently drop? |
| `aether spec-plan OLD [NEW]` | What would this edit do on each runtime? |
| `aether spec-merge [OVERLAY...]` | What is the spec for this environment (base + overlays + `--set`)? |
| `aether spec-render` | What exactly will the engine act on, and did it change? |

All of them honour `--spec FILE`, and `--json` / `-o json` for machine output.

## aether lint

```bash
aether --spec workload.yaml lint
aether --spec workload.yaml lint --deny-warnings   # CI: fail on warnings too
aether --spec workload.yaml --json lint
```

Output is `file:line: severity[CODE]: message (path)`. Exit status is 1 when
there is any error (or any warning with `--deny-warnings`).

| Code | Meaning |
| --- | --- |
| `E001` `E002` `E003` | Input guards: spec over 1 MiB, more than 16 YAML aliases (or aliases in a document over 64 KiB), more than 200,000 nodes or nesting deeper than 48. The document is refused before it is expanded. |
| `E010` `E011` `E012` | YAML/schema error (with line), `aether validate` failure, control character in a string or key. |
| `W020` | Unknown field: the key is **ignored** by Aether. Includes a "did you mean" hint. |
| `I021` | A `snake_case` spelling of a camelCase field (accepted alias). |
| `E030` `E031` | Invalid label/annotation key or label value (Kubernetes syntax). |
| `E032` `W033` | Port 0; duplicate container port. |
| `E034` | `runtime.preferred` is not listed in `runtime.allow`. |
| `E035` `W036` | `minReplicas` above `maxReplicas`; scaling enabled without metrics. |
| `W037` | Ingress enabled without a host. |
| `E038` | Request larger than the limit. |
| `I039` | Ports exposed but no health probe. |
| `W040` `I041` | A field every allowed runtime ignores / that some allowed runtime ignores. |

Line numbers come from a block-style YAML index. Keys written in flow style
(`{a: 1}`) report the line of their nearest block-style ancestor.

### Output formats, suppression and rule catalog

```bash
aether --spec workload.yaml lint --format sarif > lint.sarif   # GitHub code scanning
aether --spec workload.yaml lint --format junit  > lint.xml    # any CI test report
aether --spec workload.yaml lint --format github               # ::error file=..,line=.. annotations
aether lint --list-rules                                       # every code
aether lint --explain W020                                     # what a code means
```

`--format` is one of `text` (default), `json`, `sarif` (2.1.0, with the
properties GitHub code scanning requires and a stable fingerprint per finding),
`junit` (one test case per diagnostic; errors and warnings are failures, infos
pass) and `github` (workflow commands). Everything that comes from the spec is
escaped for the target format, so a hostile spec cannot forge extra annotations
or break the XML.

Suppress a finding on purpose, and say so in the file:

```yaml
network:
  # aether-lint: ignore=W020
  servicetype: NodePort          # the next line is exempt
  ports: []                      # aether-lint: ignore=W033   (this line only)
# aether-lint: ignore-file=I041  # whole file
```

or on the command line with `--ignore W020,I041`. The report says how many
diagnostics were suppressed. Input guards and parse/validation errors
(`E00x`, `E01x`) can never be suppressed, because they mean the rest of the
report is incomplete.

### Best-practice rules (`--best-practices`)

Opt-in hardening and reliability advice in the spirit of kube-linter and
Polaris. They are advice, not correctness, so plain `lint` never reports them.

| Code | Finding |
| --- | --- |
| B101 | no liveness probe (skipped for scheduled jobs) |
| B102 | ports exposed but no readiness probe |
| B103 | container may run as root (`runAsNonRoot` not true) |
| B104 | writable root filesystem |
| B105 | privilege escalation allowed |
| B106 | single replica and no disruption budget |
| B107 | exposed ports and no network policy |
| B108 | sidecar/init container image with no tag, `:latest`, no digest |
| B109 | default service account |

B103-B109 apply only when Kubernetes is an allowed runtime.

Cross-checked against the kube-linter check list and Polaris security checks
(see `docs/RESEARCH.md`, round 4): B101/B102 are kube-linter `no-liveness-probe`
/ `no-readiness-probe`, B103 `run-as-non-root` / Polaris `runAsRootAllowed`,
B104 `no-read-only-root-fs` / `notReadOnlyRootFilesystem`, B105
`privilege-escalation-container` / `privilegeEscalationAllowed`, B106
`minimum-three-replicas` + `pdb-*`, B107 `non-isolated-pod` / `missingNetworkPolicy`,
B108 `latest-tag`, B109 `default-service-account`.

## aether portability

```bash
aether --spec workload.yaml portability
```

Prints, for every field your spec sets, whether Podman, Kubernetes and KubeVirt
honour it, honour it partly, or ignore it, plus a per-runtime score (honoured
fields count 1, partial fields 0.5). The matrix lives in `src/portability.rs`
and is derived from what the adapters read; when an adapter starts honouring a
field, update the matrix (the tests pin the shape, not the facts).

## aether spec-plan

```bash
aether spec-plan previous.yaml            # compares against --spec
aether spec-plan old.yaml new.yaml --detailed-exit-code
```

Lists each changed field (`+` added, `-` removed, `~` changed) and its effect
per runtime:

| Effect | Meaning |
| --- | --- |
| no effect | The runtime does not read the field. |
| update in place | A separate object (Service, HPA, Ingress, ...) is patched. |
| rolling restart | Kubernetes pod-template change. |
| replace | Podman container re-created, VM restarted, or an immutable object replaced. |

With `--detailed-exit-code` the exit status is 0 (no changes), 1 (error) or 2
(changes), as in `terraform plan -detailed-exitcode`. Values under keys that look
secret (`secret`, `password`, `token`, `apikey`, `credential`, `privatekey`) are shown as
`<redacted>`; control characters in values are escaped so a spec cannot forge
extra output lines.

## aether spec-merge

One base spec, plus overlays per environment, plus quick overrides:

```bash
aether --spec base.yaml spec-merge prod.yaml
aether --spec base.yaml spec-merge prod.yaml --set requirements.memory=2Gi \
    --set 'network.ports[0].servicePort=8080' -o prod-rendered.yaml
```

Overlays are JSON Merge Patch documents (RFC 7386), the same model kustomize
overlays use for simple cases: mappings merge key by key, `null` deletes a key,
and a list replaces the base list. `--set path=value` takes a dotted path with
optional `[index]` (an index equal to the list length appends); the value is
parsed as a YAML scalar (`3` is a number, `"3"` a string) and `path=null`
deletes. The merged spec goes to stdout (or `-o FILE`) and is linted; the
lint report goes to stderr and the exit status is 1 on lint errors. Limits: 16
overlays, 64 overrides, paths up to 16 deep, and every input passes the same
size/alias/node guards as `lint`.

## aether spec-render

Like `docker compose config`: print the document the engine will act on, with
defaults filled in, keys sorted, nulls dropped and unknown keys gone.

```bash
aether --spec workload.yaml spec-render               # canonical YAML
aether --spec workload.yaml spec-render --format json # also lists per-section hashes
aether --spec workload.yaml spec-render --hash        # sha256:... for CI change detection
```

The hash ignores comments, key order and anything the engine ignores. Per
section hashes show which part of a spec changed. Secret-looking values are
printed as `***`, but the hash covers the real values, so rotating a secret
still changes it. Treat the hash itself as non-public for specs whose secrets
are low-entropy.

### Auto-fix (`--fix`, `--diff`)

`aether lint --fix` applies only safe, textual edits and keeps comments and
layout: a misspelt key with a "did you mean" hint is renamed (`W020`; the old
key was being ignored, so this *does* change behaviour and is reported as
such) and a snake_case alias becomes its canonical camelCase key (`I021`,
behaviour-neutral). `--diff` shows the edits and changes nothing. If the edited
spec would have more errors than before, nothing is written. Best-practice
rules are never auto-fixed.

## aether spec-policy

Policy as code: organisation rules live in a YAML file and are evaluated
against the spec as written, so every finding has a source line. See
`examples/policy/org-baseline.yaml`.

```yaml
rules:
  - id: ORG003
    description: memory request stays below 8Gi
    path: requirements.memory
    op: lte
    value: 8Gi
    severity: error            # error (default) | warning | info
    when: { path: runtime.preferred, op: equals, value: kube }   # optional
    skip_if_missing: false     # optional
    message: custom text       # optional
```

Operators: `exists`, `absent`, `equals`, `not_equals`, `in`, `not_in`,
`matches` / `not_matches` (glob `*` `?`), `lt`, `lte`, `gt`, `gte` (numbers and
quantities such as `500m`, `256Mi`), `min_len`, `max_len`. Paths use dots,
`[N]` and `[*]` (every list element; quote the path in flow style, since a bare `*` starts a YAML alias). A missing value fails every operator
except `absent`, `not_equals`, `not_in` and `not_matches`. Formats: `text`,
`json`, `junit`, `github`; exit 1 on errors (or warnings with
`--deny-warnings`). Unknown rule keys, duplicate ids and bad paths are rejected.

## aether spec-lock

Offline drift detection against a committed baseline. `spec-lock write` stores
the canonical hashes (overall and per section, the same as `spec-render`) in
`<spec>.lock.json`; the lock holds no spec values. `spec-lock check` exits 0
with no drift, 2 on drift (naming the changed, added and removed sections) and
1 on errors such as a malformed lock. `--ignore requirements,scaling` leaves
sections out of the comparison. For live state use `aether diff`.

## aether manifest-check

kubeconform-style validation of Kubernetes manifests (rendered, Helm or
kustomize output), offline. Each document is parsed into the typed
`k8s-openapi` object for its `apiVersion`/`kind`: wrong types, missing
required fields, bad base64 in Secrets and apiVersions the API no longer
serves (`extensions/v1beta1` Ingress) are invalid. `--strict` also reports
unknown fields with their path. Multi-document streams, `kind: List` and stdin
(`-`) work. Built-in kinds only (Pod, Service, ConfigMap, Secret, Namespace,
ServiceAccount, PVC, ResourceQuota, LimitRange, Deployment, StatefulSet,
DaemonSet, ReplicaSet, Job, CronJob, Ingress, NetworkPolicy, HPA v2, PDB, RBAC
kinds); custom resources (KubeVirt, Cilium, ...) are errors unless
`--ignore-missing-schemas`, or skipped with `--skip Kind`. Formats `text`,
`json`, `junit`; exit 1 on invalid or error. The schema is the one bundled
with the `k8s-openapi` crate; CEL rules and admission are not evaluated.

## aether spec-ci

`aether spec-ci github|gitlab|pre-commit` prints a ready-to-paste config that
runs `lint --best-practices` and `spec-lock check`. The spec path comes from
`--spec` and is restricted to `[A-Za-z0-9._/-]`.

## API

The same checks are available over HTTP (Operator or Admin role, like other
POST endpoints):

| Endpoint | Body | Returns |
| --- | --- | --- |
| `POST /api/spec/lint` | `{"spec": "<yaml>", "bestPractices": false, "ignore": [], "format": "json", "file": "spec.yaml"}` | lint report (`format` `sarif`, `junit`, `github` return that rendering in `data`) |
| `GET /api/spec/lint/rules`, `GET /api/spec/lint/rules/{code}` | - | rule catalog / one rule |
| `POST /api/spec/merge` | `{"base": "<yaml>", "overlays": ["<yaml>"], "set": ["a.b=1"]}` | `{spec, lint}` |
| `POST /api/spec/render` | `{"spec": "<yaml>"}` | `{hash, sections, spec}` |
| `POST /api/spec/portability` | `{"spec": "<yaml>"}` | portability report |
| `POST /api/spec/plan` | `{"old": "<yaml>", "new": "<yaml>"}` | plan |
| `POST /api/spec/fix` | `{"spec": "<yaml>"}` | `{fixed, changed, applied, skipped, remaining}` (nothing is stored) |
| `POST /api/spec/policy` | `{"spec": "<yaml>", "rules": "<yaml>"}` | policy report |
| `POST /api/spec/lock` | `{"spec": "<yaml>"}` | lock |
| `POST /api/spec/lock/check` | `{"spec": "<yaml>", "lock": "<json>", "ignore": []}` | drift result |
| `POST /api/spec/manifest-check` | `{"manifests": "<yaml>", "strict": false, "ignoreMissingSchemas": false, "skip": []}` | `{results, summary}` |
| `GET /api/spec/ci/{provider}?spec=path` | - | `{provider, snippet}` |

Request bodies larger than 1 MiB per spec are refused.

## Golden tests for renderers

`src/adapters/golden_tests.rs` renders the Kubernetes Deployment/Service and the
KubeVirt VirtualMachine for shipped examples and compares them with
`tests/golden/*.yaml`. After an intentional renderer change run
`AETHER_UPDATE_GOLDEN=1 cargo test --lib golden` and review the diff.
