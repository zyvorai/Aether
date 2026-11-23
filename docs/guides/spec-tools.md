---
hero:
  eyebrow: GUIDES
  title: Spec tools - lint, portability, plan
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

All three honour `--spec FILE`, and `--json` / `-o json` for machine output.

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

## API

The same checks are available over HTTP (Operator or Admin role, like other
POST endpoints):

| Endpoint | Body | Returns |
| --- | --- | --- |
| `POST /api/spec/lint` | `{"spec": "<yaml>"}` | lint report |
| `POST /api/spec/portability` | `{"spec": "<yaml>"}` | portability report |
| `POST /api/spec/plan` | `{"old": "<yaml>", "new": "<yaml>"}` | plan |

Request bodies larger than 1 MiB per spec are refused.

## Golden tests for renderers

`src/adapters/golden_tests.rs` renders the Kubernetes Deployment/Service and the
KubeVirt VirtualMachine for shipped examples and compares them with
`tests/golden/*.yaml`. After an intentional renderer change run
`AETHER_UPDATE_GOLDEN=1 cargo test --lib golden` and review the diff.
