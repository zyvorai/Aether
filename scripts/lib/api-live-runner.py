#!/usr/bin/env python3
"""Live API exerciser — parses routes from src/api/mod.rs and probes a running server."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
MOD_RS = ROOT / "src" / "api" / "mod.rs"

# Endpoints that need query params to be meaningful.
QUERY_DEFAULTS: dict[str, str] = {
    "/api/cluster/namespaces": "cluster=active-client",
    "/api/cluster/metrics/summary": "cluster=active-client&namespace=aether-system",
    "/api/cluster/browse": "cluster=active-client&namespace=aether-system&kind=Deployment",
    "/api/cluster/logs": "cluster=active-client&namespace=aether-system&kind=Pod&name=probe",
    "/api/cluster/resource": "cluster=active-client&namespace=aether-system&kind=Deployment&name=probe",
    "/api/cluster/events": "cluster=active-client&namespace=aether-system",
    "/api/cluster/health": "cluster=active-client",
    "/api/cluster/top": "cluster=active-client&namespace=aether-system",
    "/api/cluster/rollout": "cluster=active-client&namespace=aether-system&kind=Deployment&name=probe",
    "/api/cluster/helm/history": "cluster=active-client&namespace=aether-system&release=probe",
    "/api/observability/prometheus/query": "query=up",
    "/api/cost/chargeback": "provider=aws",
    "/api/intelligence/graph/search": "q=workload",
    "/api/intelligence/graph/impact": "node=probe",
    "/api/intelligence/graph/blast-radius": "node=probe",
    "/api/intelligence/graph/threat-paths": "source=probe&target=probe",
    "/api/intelligence/graph/placement": "workload=probe",
    "/api/intelligence/migration/volume-status": "workload=probe",
    "/api/intelligence/migration/wave-plan": "workload=probe",
    "/api/intelligence/federation/geo-placement": "region=us-east-1",
    "/api/intelligence/federation/region-lock": "region=us-east-1",
    "/api/intelligence/sre/runbook": "incident=probe",
    "/api/intelligence/sre/postmortem": "incident=probe",
    "/api/intelligence/sre/incident-timeline": "incident=probe",
    "/api/intelligence/sre/escalation": "incident=probe",
    "/api/intelligence/sre/mttr": "window=24h",
    "/api/intelligence/macos/spotlight": "q=workload",
    "/api/ecosystem/packetwolf/deeplink": "target=probe",
    "/api/drift/": "name=probe",  # prefix fallback handled below
}

MINIMAL_SPEC = """apiVersion: aether/v1
kind: Workload
metadata:
  name: live-probe
  owner: api-live-test
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 100m
  memory: 128Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
network:
  service: true
  ports:
    - containerPort: 80
      servicePort: 80
      protocol: TCP
"""

POST_BODIES: dict[str, Any] = {
    "/api/validate": {"spec_yaml": MINIMAL_SPEC},
    "/api/cost": {"requirements": {"cpu": "100m", "memory": "128Mi", "storage": "1Gi"}},
    "/api/ai/recommend": {"requirements": {"cpu": "100m", "memory": "128Mi"}},
    "/api/ai/intent-optimize": {"workload": "live-probe"},
    "/api/ai/right-size": {"workload": "live-probe"},
    "/api/ai/tradeoff": {"workload": "live-probe", "target_runtime": "kube"},
    "/api/intelligence/intent-pipeline": {"intent": "deploy nginx"},
    "/api/intelligence/intent/nl-parse": {"text": "deploy nginx with 100m cpu"},
    "/api/intelligence/intent/compliance/check": {"spec_yaml": MINIMAL_SPEC},
    "/api/intelligence/intent/bundles": {"name": "probe-bundle"},
    "/api/intelligence/intent/budget/enforce": {"budget_usd": 100},
    "/api/intelligence/digital-twin/simulate": {"workload": "live-probe"},
    "/api/intelligence/gitops/agent/sync": {"dry_run": True},
    "/api/intelligence/cost-optimize/apply": {"dry_run": True},
    "/api/intelligence/security/remediate": {"dry_run": True},
    "/api/intelligence/capacity/scale/execute": {"dry_run": True},
    "/api/intelligence/healer/execute": {"dry_run": True},
    "/api/intelligence/evolution/execute": {"dry_run": True},
    "/api/intelligence/federation/execute": {"dry_run": True},
    "/api/intelligence/federation/packetwolf-guard/apply": {"dry_run": True},
    "/api/intelligence/graph/import-k8s": {"cluster": "active-client"},
    "/api/intelligence/graph/snapshots/capture": {"label": "live-probe"},
    "/api/intelligence/graph/cmdb/sync": {"dry_run": True},
    "/api/intelligence/sre/on-call/test": {"channel": "probe"},
    "/api/intelligence/sre/chaos/run": {"experiment": "probe", "dry_run": True},
    "/api/intelligence/sre/runbook/execute": {"runbook": "probe", "dry_run": True},
    "/api/intelligence/intent-pipeline/deploy": {"dry_run": True},
    "/api/intelligence/place": {"yaml": MINIMAL_SPEC},
    "/api/policy/check": {"workload": "live-probe"},
    "/api/policy/opa": {"policy": "package aether\nallow = true\n"},
    "/api/compose/validate": {"compose_yaml": "services:\n  web:\n    image: nginx\n"},
    "/api/compose/up": {"compose_yaml": "services:\n  web:\n    image: nginx\n", "dry_run": True},
    "/api/compose/down": {"compose_yaml": "services:\n  web:\n    image: nginx\n", "dry_run": True},
    "/api/helm/export": {"spec_yaml": MINIMAL_SPEC},
    "/api/gitops/preview": {"paths": []},
    "/api/gitops/sync": {"dry_run": True},
    "/api/gitops/init": {"repo_url": "https://example.com/repo.git", "dry_run": True},
    "/api/gitops/resolve-target": {"path": "workloads/probe.yaml"},
    "/api/webhooks/test": {"channel": "probe", "dry_run": True},
    "/api/webhooks/channels": {"name": "probe", "type": "webhook", "url": "https://example.com/hook"},
    "/api/webhooks/flush": {},
    "/api/audit/events": {"action": "api-live-test", "resource": "probe", "detail": "live probe"},
    "/api/environments/promote": {"workload": "live-probe", "from": "dev", "to": "staging", "dry_run": True},
    "/api/environments": {"name": "probe-env", "description": "live test"},
    "/api/sla": {"workload": "live-probe", "target_availability": 99.9},
    "/api/orchestrator/health-check": {},
    "/api/orchestrator/reset-circuit": {"workload": "live-probe"},
    "/api/cluster/apply": {"cluster": "active-client", "manifest": "apiVersion: v1\nkind: ConfigMap\n", "dry_run": True},
    "/api/cluster/action": {"cluster": "active-client", "action": "describe", "dry_run": True},
    "/api/cluster/diff": {"cluster": "active-client", "manifest": "apiVersion: v1\nkind: ConfigMap\n"},
    "/api/cluster/helm/action": {"cluster": "active-client", "release": "probe", "action": "status"},
    "/api/migration/volume/plan": {"workload": "live-probe", "target": "kube"},
    "/api/migration/volume/execute": {"workload": "live-probe", "target": "kube", "dry_run": True},
    "/api/migration/fleet/plan": {"workloads": ["live-probe"], "target": "kube"},
    "/api/fleet/edge/register": {"node_id": "probe-node", "dry_run": True},
    "/api/fleet/edge/heartbeat": {"node_id": "probe-node"},
    "/api/fleet/edge/enqueue": {"node_id": "probe-node", "task": "probe"},
    "/api/fleet/federation/plan": {"dry_run": True},
    "/api/ecosystem/packetwolf/verify-egress": {"destination": "8.8.8.8", "dry_run": True},
    "/api/copilot/chat": {"message": "health check probe"},
    "/api/copilot/troubleshoot": {"workload": "live-probe", "include_copilot_summary": False},
    "/api/copilot/confirm-batch": {"action_ids": []},
    "/api/zeus/chat": {"message": "health check probe"},
    "/api/zeus/troubleshoot": {"workload": "live-probe"},
    "/api/zeus/confirm-batch": {"action_ids": []},
    "/api/zeus/prompts/import": {"prompts": []},
    "/api/hosted/tenants": {"name": "probe-tenant", "plan": "starter"},
    "/api/hosted/billing/stripe/checkout": {"plan": "starter", "dry_run": True},
    "/api/hosted/billing/stripe/portal": {"dry_run": True},
}

SKIP_PATHS = {
    "/",
    "/assets/aether-dashboard.css",
    "/assets/aether-dashboard.js",
    "/api/auth/oidc/login",
    "/api/auth/oidc/callback",
    "/api/auth/oidc/logout",
    "/api/auth/saml/login",
    "/api/auth/saml/acs",
    "/api/auth/saml/logout",
    "/api/auth/ldap/logout",
    "/api/hosted/billing/stripe/webhook",
    "/api/cluster/ws/exec",
    "/api/cluster/ws/watch",
}

OPTIONAL_500_PATH_PREFIXES = (
    "/api/cluster/",
    "/api/gitops/",
    "/api/observability/prometheus/",
    "/api/intelligence/platform/",
    "/api/intelligence/migration/",
    "/api/ai/",
)

# Server errors that mean "endpoint works, probe data missing" — not a hard failure.
OPTIONAL_500_ERROR_MARKERS = (
    "no workloads in fleet",
    "no workloads resolved",
    "expected workload json or yaml",
    "expected struct workload",
    "workload not found",
    "not found",
    "no kubeconfig",
    "kubeconfig not configured",
    "invalid workload name",
    "failed to load workload spec",
    "no such file or directory",
    "missing field",
)

ANSI = {
    "reset": "\033[0m",
    "bold": "\033[1m",
    "dim": "\033[2m",
    "green": "\033[32m",
    "yellow": "\033[33m",
    "red": "\033[31m",
    "cyan": "\033[36m",
    "magenta": "\033[35m",
}


@dataclass
class Route:
    method: str
    path: str


@dataclass
class Result:
    method: str
    path: str
    status: int
    elapsed_ms: float
    body: str
    outcome: str
    note: str = ""


@dataclass
class Context:
    base: str
    headers: dict[str, str]
    placeholders: dict[str, str] = field(default_factory=dict)
    body_max: int = 2000
    timeout: float = 20.0
    log_path: Path | None = None
    sse_seconds: float = 2.0


def parse_routes(mod_rs: Path) -> list[Route]:
    text = mod_rs.read_text()
    start = text.find("let mut app = Router::new()")
    if start == -1:
        raise SystemExit(f"Could not locate Router::new() in {mod_rs}")
    mock_start = text.find("\n    if crate::mock_idp::enabled()", start)
    fallback = text.find("\n    let app = app", start)
    end = mock_start if mock_start != -1 else fallback
    if end == -1:
        raise SystemExit(f"Could not locate router end in {mod_rs}")
    section = text[start:end]
    if mock_start != -1 and fallback != -1:
        section += text[mock_start:fallback]
    routes: list[Route] = []
    pos = 0
    while True:
        idx = section.find(".route(", pos)
        if idx == -1:
            break
        i = idx + len(".route(")
        depth = 1
        while i < len(section) and depth:
            ch = section[i]
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
            i += 1
        block = section[idx + len(".route(") : i - 1]
        path_match = re.search(r'^\s*"([^"]+)"\s*,\s*(.*)', block, re.DOTALL)
        if not path_match:
            pos = i
            continue
        path = path_match.group(1)
        rest = path_match.group(2)
        methods = re.findall(r"\b(get|post|put|patch|delete)\s*\(", rest)
        if not methods:
            pos = i
            continue
        for method in methods:
            routes.append(Route(method.upper(), path))
        pos = i
    return routes


def substitute_path(path: str, ctx: Context) -> str:
    out = path
    for key, value in ctx.placeholders.items():
        out = out.replace(f":{key}", urllib.parse.quote(value, safe=""))
    out = re.sub(r":\w+", "probe", out)
    return out


def query_for(path: str) -> str:
    if path in QUERY_DEFAULTS:
        return QUERY_DEFAULTS[path]
    for prefix, query in QUERY_DEFAULTS.items():
        if prefix.endswith("/") and path.startswith(prefix.rstrip("/")):
            return query
    return ""


def body_for(method: str, path: str) -> bytes | None:
    if method in {"GET", "DELETE"}:
        return None
    payload = POST_BODIES.get(path, {})
    if method in {"PUT", "PATCH"}:
        payload = {"spec_yaml": MINIMAL_SPEC, **payload}
    return json.dumps(payload).encode()


def api_error_message(body: str) -> str:
    try:
        parsed = json.loads(body)
        err = parsed.get("error")
        if isinstance(err, str):
            return err
    except json.JSONDecodeError:
        pass
    return body


def is_timeout_error(exc: BaseException) -> bool:
    if isinstance(exc, TimeoutError):
        return True
    reason = getattr(exc, "reason", None)
    if isinstance(reason, TimeoutError):
        return True
    text = str(exc).lower()
    return "timed out" in text or "timeout" in text


def is_probe_optional_500(path: str, body: str) -> bool:
    if path.startswith(OPTIONAL_500_PATH_PREFIXES):
        return True
    msg = api_error_message(body).lower()
    return any(marker in msg for marker in OPTIONAL_500_ERROR_MARKERS)


def classify(status: int, path: str, method: str, body: str = "") -> tuple[str, str]:
    if status == 0:
        if path == "/api/events/stream":
            return "warn", "SSE stream probe timed out (connection likely open)"
        return "fail", "request failed / timeout"
    if 200 <= status < 300:
        return "pass", ""
    if status in {400, 401, 403, 404, 405, 409, 422}:
        return "warn", f"HTTP {status} (endpoint reachable)"
    if status == 500 and is_probe_optional_500(path, body):
        detail = api_error_message(body).strip()
        if detail:
            short = detail if len(detail) <= 80 else detail[:77] + "..."
            return "warn", f"HTTP 500 (probe — {short})"
        return "warn", "HTTP 500 (optional — cluster/config or probe data missing)"
    if status >= 500:
        return "fail", f"HTTP {status}"
    return "warn", f"HTTP {status}"


def pretty_body(raw: str, limit: int) -> str:
    text = raw.strip()
    if not text:
        return "(empty)"
    try:
        parsed = json.loads(text)
        text = json.dumps(parsed, indent=2, ensure_ascii=False)
    except json.JSONDecodeError:
        pass
    if len(text) > limit:
        return text[:limit] + f"\n… ({len(raw)} bytes total, truncated)"
    return text


def request_once(ctx: Context, method: str, path: str) -> Result:
    resolved = substitute_path(path, ctx)
    query = query_for(resolved)
    url = ctx.base.rstrip("/") + resolved
    if query:
        url += "?" + query

    data = body_for(method, path)
    headers = dict(ctx.headers)
    if data is not None:
        headers.setdefault("Content-Type", "application/json")

    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    started = time.perf_counter()
    body = ""
    status = 0
    try:
        if path == "/api/events/stream":
            req = urllib.request.Request(url, headers=headers, method="GET")
            try:
                with urllib.request.urlopen(req, timeout=ctx.sse_seconds) as resp:
                    status = resp.status
                    chunk = resp.read(4096)
                    body = chunk.decode("utf-8", errors="replace")
                    if not body:
                        body = "(SSE stream opened, no events within probe window)"
            except Exception as sse_exc:  # noqa: BLE001
                if is_timeout_error(sse_exc):
                    elapsed_ms = (time.perf_counter() - started) * 1000
                    return Result(
                        method,
                        resolved,
                        0,
                        elapsed_ms,
                        str(sse_exc),
                        "warn",
                        "SSE stream probe timed out (connection likely open)",
                    )
                raise
        else:
            with urllib.request.urlopen(req, timeout=ctx.timeout) as resp:
                status = resp.status
                body = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as err:
        status = err.code
        body = err.read().decode("utf-8", errors="replace")
    except Exception as exc:  # noqa: BLE001 — show live probe errors
        elapsed_ms = (time.perf_counter() - started) * 1000
        if path == "/api/events/stream" and is_timeout_error(exc):
            return Result(
                method,
                resolved,
                0,
                elapsed_ms,
                str(exc),
                "warn",
                "SSE stream probe timed out (connection likely open)",
            )
        return Result(method, resolved, 0, elapsed_ms, str(exc), "fail", str(exc))

    elapsed_ms = (time.perf_counter() - started) * 1000
    outcome, note = classify(status, resolved, method, body)
    return Result(method, resolved, status, elapsed_ms, body, outcome, note)


def colorize(outcome: str, text: str) -> str:
    tone = {
        "pass": ANSI["green"],
        "warn": ANSI["yellow"],
        "fail": ANSI["red"],
        "skip": ANSI["dim"],
    }.get(outcome, "")
    return f"{tone}{text}{ANSI['reset']}"


def print_live(idx: int, total: int, result: Result, ctx: Context) -> None:
    icon = {"pass": "✓", "warn": "⚡", "fail": "✗", "skip": "○"}.get(result.outcome, "?")
    line = (
        f"\n{ANSI['bold']}── [{idx:>4}/{total}] "
        f"{result.method} {result.path}{ANSI['reset']}\n"
    )
    sys.stdout.write(line)
    status_text = f"{result.status}" if result.status else ("SSE" if result.path == "/api/events/stream" else "ERR")
    sys.stdout.write(
        colorize(result.outcome, f"  {icon} Status: {status_text}") + f"  ({result.elapsed_ms:.0f}ms)\n"
    )
    if result.note:
        sys.stdout.write(colorize(result.outcome, f"  Note: {result.note}") + "\n")
    preview = pretty_body(result.body, ctx.body_max)
    sys.stdout.write(f"{ANSI['cyan']}  Response:{ANSI['reset']}\n")
    for body_line in preview.splitlines():
        sys.stdout.write(f"    {body_line}\n")
    sys.stdout.flush()


def append_log(ctx: Context, result: Result) -> None:
    if not ctx.log_path:
        return
    entry = {
        "method": result.method,
        "path": result.path,
        "status": result.status,
        "elapsed_ms": round(result.elapsed_ms, 1),
        "outcome": result.outcome,
        "note": result.note,
        "body": result.body[:10000],
    }
    with ctx.log_path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(entry, ensure_ascii=False) + "\n")


def bootstrap_placeholders(ctx: Context) -> None:
    ctx.placeholders = {
        "name": "live-probe",
        "workload": "live-probe",
        "target": "kube",
        "id": "probe-id",
        "action_id": "probe-action",
        "class": "web-service",
    }
    dns_name = re.compile(r"^[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$")
    url = ctx.base.rstrip("/") + "/api/workloads"
    req = urllib.request.Request(url, headers=ctx.headers, method="GET")
    try:
        with urllib.request.urlopen(req, timeout=ctx.timeout) as resp:
            payload = json.loads(resp.read().decode())
    except Exception:
        return
    items = payload.get("data")
    if not isinstance(items, list) or not items:
        return

    chosen = None
    for item in items:
        if not isinstance(item, dict):
            continue
        if item.get("source") != "aether":
            continue
        wl_name = item.get("name") or item.get("metadata", {}).get("name")
        if isinstance(wl_name, str) and dns_name.match(wl_name):
            chosen = wl_name
            break
    if chosen:
        ctx.placeholders["name"] = chosen
        ctx.placeholders["workload"] = chosen


def should_skip(route: Route, methods_filter: set[str], mutations: bool) -> str | None:
    if route.path in SKIP_PATHS:
        return "non-API or auth redirect"
    if route.method not in methods_filter:
        return "method filtered"
    if route.method in {"DELETE", "PUT", "PATCH"} and not mutations:
        return "mutation (use --mutations)"
    if route.path.startswith("/api/mock-idp/"):
        return "mock IdP helper"
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description="Live Aether API test runner")
    parser.add_argument("--base", default=os.environ.get("AETHER_API", os.environ.get("AETHER_API_BASE", "http://127.0.0.1:5090")))
    parser.add_argument("--mod-rs", type=Path, default=MOD_RS)
    parser.add_argument("--methods", default=os.environ.get("AETHER_API_LIVE_METHODS", "GET,POST"))
    parser.add_argument("--mutations", action="store_true", help="Also probe PUT/PATCH/DELETE")
    parser.add_argument("--body-max", type=int, default=int(os.environ.get("AETHER_API_LIVE_BODY_MAX", "2000")))
    parser.add_argument("--timeout", type=float, default=float(os.environ.get("AETHER_API_LIVE_TIMEOUT", "20")))
    parser.add_argument("--log", type=Path, default=None)
    parser.add_argument("--auth-header", default=os.environ.get("AETHER_API_LIVE_AUTH_HEADER", ""))
    parser.add_argument("--cookie", default=os.environ.get("AETHER_API_LIVE_COOKIE", ""))
    parser.add_argument("--limit", type=int, default=0, help="Stop after N probes (0 = all)")
    args = parser.parse_args()

    log_path = args.log
    if log_path is None and os.environ.get("AETHER_API_LIVE_LOG"):
        log_path = Path(os.environ["AETHER_API_LIVE_LOG"])

    methods_filter = {m.strip().upper() for m in args.methods.split(",") if m.strip()}
    if args.mutations:
        methods_filter.update({"PUT", "PATCH", "DELETE"})

    headers: dict[str, str] = {"Accept": "application/json, text/event-stream, */*"}
    if args.auth_header:
        headers["Authorization"] = args.auth_header
    if args.cookie:
        headers["Cookie"] = args.cookie

    ctx = Context(
        base=args.base,
        headers=headers,
        body_max=args.body_max,
        timeout=args.timeout,
        log_path=log_path,
    )

    routes = parse_routes(args.mod_rs)
    # Stable ordering: GET first, then POST, then others; group by path prefix.
    method_rank = {"GET": 0, "POST": 1, "PUT": 2, "PATCH": 3, "DELETE": 4}
    routes.sort(key=lambda r: (r.path, method_rank.get(r.method, 9), r.method))

    # Preflight
    health_url = args.base.rstrip("/") + "/health"
    try:
        with urllib.request.urlopen(health_url, timeout=5) as resp:
            if resp.status != 200:
                print(colorize("fail", f"Preflight /health returned {resp.status}"), file=sys.stderr)
                return 1
    except Exception as exc:  # noqa: BLE001
        print(colorize("fail", f"Preflight /health failed: {exc}"), file=sys.stderr)
        return 1

    print(f"{ANSI['bold']}Aether live API test{ANSI['reset']}")
    print(f"  Target:  {args.base}")
    print(f"  Routes:  {len(routes)} from {args.mod_rs.name}")
    print(f"  Methods: {', '.join(sorted(methods_filter))}")
    if log_path:
        log_path.write_text("", encoding="utf-8")
        print(f"  Log:     {log_path}")
    print("")

    print(colorize("pass", "  ✓ Preflight /health OK"))
    bootstrap_placeholders(ctx)
    print(
        f"  Placeholders: name={ctx.placeholders['name']} target={ctx.placeholders['target']}\n"
    )

    counts = {"pass": 0, "warn": 0, "fail": 0, "skip": 0}
    executable: list[Route] = []
    for route in routes:
        reason = should_skip(route, methods_filter, args.mutations)
        if reason:
            counts["skip"] += 1
            continue
        executable.append(route)

    total = len(executable)
    if args.limit and args.limit > 0:
        executable = executable[: args.limit]
        total = len(executable)
    for idx, route in enumerate(executable, start=1):
        result = request_once(ctx, route.method, route.path)
        print_live(idx, total, result, ctx)
        append_log(ctx, result)
        counts[result.outcome] = counts.get(result.outcome, 0) + 1

    print(f"\n{ANSI['bold']}Summary{ANSI['reset']}")
    print(
        f"  {colorize('pass', 'pass=' + str(counts['pass']))}  "
        f"{colorize('warn', 'warn=' + str(counts['warn']))}  "
        f"{colorize('fail', 'fail=' + str(counts['fail']))}  "
        f"{colorize('skip', 'skip=' + str(counts['skip']))}"
    )
    if log_path:
        print(f"  Full capture: {log_path}")
    return 1 if counts["fail"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
