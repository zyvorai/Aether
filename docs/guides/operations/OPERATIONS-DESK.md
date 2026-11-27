# Operations desk

What needs attention, what changed, what broke, and what an action would do before you take it. The same engine backs the CLI, the REST API and the **Operations Desk** page in the dashboard (`/operations`).

Everything here comes from records Aether writes under `~/.aether`. Nothing is estimated; where there is no record, the output says so.

## What is recorded

Every runtime adapter Aether creates is wrapped in a change journal, so a deploy, update, stop or delete is recorded the same way whether it came from the CLI, the API, a compose file, a migration or the self-healing loop.

| File | Contents |
|---|---|
| `audit.json` | Success and failure of each change, with the actor (`cli:<user>`, `api:<key name or user>`, `serve`) |
| `revisions/<name>.json` | The spec that was applied, when, by whom, why, and the engine's decision for it (last 100) |
| `health.json` | Raw health checks (last 1440 per workload) |
| `availability.json` | Hourly ready/total counters (31 days) |
| `incidents.json` | Incidents, acknowledgements, notes, planned downtime |
| `holds.json` | Change holds |

Revision files hold the full spec, including any secret values in it. They are written owner-only (`0600`), and every command, API response and report redacts secret-looking values.

Not recorded: changes made outside Aether (`kubectl`, `podman`, the cluster browser's raw actions), and anything that happened before you upgraded to a version with the journal.

## Health checks and incidents

`aether serve` checks every managed workload every 60 seconds (`AETHER_OBSERVE_SECS`; `0` turns it off). Without a server, run `aether health collect` from a timer.

- An incident opens after 2 consecutive not-ready checks (`AETHER_INCIDENT_THRESHOLD`) and resolves on the next ready one. Opening and resolving emit events, which your notification channels receive according to their severity filter.
- A workload you stopped or deleted through Aether is expected to be down: no incident, and those checks do not count against availability. A failed deploy or update is not planned downtime and is watched again.
- A status that cannot be read (runtime missing, cluster unreachable) is reported as "cannot be observed", not as downtime.
- Times are check times. An outage shorter than the interval can be missed.

```bash
aether incidents                      # list, newest first
aether incidents show 3               # timeline and the changes before it
aether incidents ack 3
aether incidents note 3 "restarted the node"
aether incidents report 3 --out incident-3.md
```

The "changes before it" list is every change to the workload and its declared dependencies in the 30 minutes before the first failed check. It is a correlation in time, not a diagnosis. The report is a draft with the analysis left for you to write.

Mean time to recovery (also on the SRE reliability panel) is measured over incidents that ended with a ready check. With none, it reads "no data".

## Attention and changes

```bash
aether attention                      # open incidents, failed operations, unapplied spec edits, stale health, holds
aether changes --since 24h            # one feed: deploys, operations, warnings, incidents, notes
aether changes --since 7d --workload web --json
```

## Explain a workload

```bash
aether explain web
```

Why it runs where it does (each engine rule and whether it applied), the spec in effect, health, incidents, dependencies, spec fields the runtime ignores, and recent changes. It reads records only and does not query the runtime.

## History and rollback

```bash
aether history web                    # applied revisions
aether history web --diff 3 5         # field changes with the effect per runtime
aether history web --show 3           # the spec, secrets redacted
aether rollback web                   # previous revision with a different spec
aether rollback web --revision 3
```

Rollback applies the spec stored in the revision, so it does not matter what has happened to your spec file since. After a failed change, a plain `aether rollback` applies the last good revision. Aether writes the applied spec to `~/.aether/specs/rollback/<name>.yaml` and points the workload at it; your own file is not touched.

Limits: only the spec is rolled back. Images are rebuilt from the build context and volume data is not restored. A revision that ran on a different runtime is refused (migrate back first). `--version N` still uses the older snapshots, which only point at a spec file.

## Preview before you act

```bash
aether preview web --update edited.yaml
aether preview web --delete           # also --stop, --restart, --migrate kube, --rollback [REVISION]
```

The verdict is **blocked** (a hold, an invalid spec, a policy or quota refusal, an impossible target), **caution** (read the warnings) or **clear**. The exit status is `2` when blocked. A preview changes nothing and does not contact the runtime, so it cannot see changes made outside Aether.

## Change holds

```bash
aether hold set web --reason "payroll run" --for 8h     # omit --for to hold until released
aether hold list
aether hold release web
```

While a workload is held, Aether refuses to deploy, update, start, stop, restart, scale, migrate, roll back or delete it, from the CLI, the API (HTTP 409), compose files, the autoscaler and the self-healing loop. An unreadable hold file blocks changes rather than allowing them.

A hold is a guard inside Aether. It does not stop `kubectl`, `podman`, the cluster browser's raw resource and Helm actions, or `aether move`.

## Operations report

```bash
aether ops-report --since 7d                       # Markdown
aether ops-report --since 30d --format html --out report.html
aether ops-report --format json
```

Availability per workload, incidents, open items, changes and an audit integrity check. Availability is the share of recorded checks that found a workload ready, not user-facing uptime.

The audit integrity check recomputes an unkeyed SHA-256 per event. It detects accidental or careless edits, not an attacker who can rewrite the file and recompute the hashes. For tamper resistance, ship events to an external system with `AETHER_AUDIT_WEBHOOK_URL`.

## REST API

| Method and path | Purpose |
|---|---|
| `GET /api/attention` | Attention queue |
| `GET /api/changes?since=&workload=&limit=` | Change feed |
| `GET /api/incidents?workload=&open=` | Incidents and recovery time |
| `GET /api/incidents/{id}` , `.../report` | Detail; Markdown draft |
| `POST /api/incidents/{id}/ack`, `.../notes` | Acknowledge; add a note (`{"text": "..."}`) |
| `POST /api/observe` | Check every workload now |
| `GET /api/workloads/{name}/explain` | Explanation |
| `GET /api/workloads/{name}/revisions`, `.../revisions/{id}` | History |
| `GET /api/workloads/{name}/revision-diff?from=&to=` | Compare |
| `POST /api/workloads/{name}/preview` | `{"action": "update", "yaml": "..."}`, `delete`, `stop`, `restart`, `{"action": "migrate", "target": "kube"}`, `{"action": "rollback", "revision": 3}` |
| `POST /api/workloads/{name}/rollback` | `{"revision": 3}` or `{}` |
| `GET /api/holds`, `POST /api/holds`, `POST /api/holds/release` | Holds (`{"workload", "reason", "duration"}`) |
| `GET /api/ops-report?since=&format=json\|markdown\|html` | Report |

Viewer keys can read all of the `GET` routes. Preview, rollback, holds, acknowledgements and notes are `POST`s and need Operator or Admin.
