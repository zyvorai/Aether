# Status board

One tile per managed workload, live, for a browser tab or a shared monitor: **Status Board** in the dashboard (`/board`).

It draws only what Aether recorded under `~/.aether`: health checks, incidents, holds, revisions and the dependency graph (see the [operations desk](OPERATIONS-DESK.md)). Nothing is estimated. A period without a check is drawn empty and called "no data"; it never counts as healthy.

## Reading a tile

| State | Meaning |
|---|---|
| Ready / Not ready | What the last check found |
| Stale | The last check is too old to describe the present: older than five check intervals (at least 3 minutes), or 15 minutes when the server does not check |
| Cannot be observed | The runtime did not answer. This is not counted as downtime |
| Stopped on purpose | Stopped or deleted through Aether |
| No data | No check has been recorded |
| Unknown | The health records or incidents could not be read |

Open incidents and change holds appear on the tile. A workload with an open incident stays red and sorts first until a ready check resolves it.

The **uptime strip** has one cell per period, oldest on the left: ready, partly ready (some checks failed), not ready, stopped on purpose, or no data (an empty slot). Stripes mean part of that period has no checks. The percentage under it is the share of recorded checks that found the workload ready, with planned downtime left out. It is not user-facing uptime.

| Window | Cell | Source |
|---|---|---|
| 1h | 2 minutes | individual checks (the last 1440 per workload are kept) |
| 6h | 10 minutes | individual checks |
| 24h | 1 hour | hourly counters (31 days are kept) |
| 7d | 3 hours | hourly counters |
| 30d | 1 day (UTC) | hourly counters |

Whether a stopped period was on purpose is reconstructed from the audit trail: from a successful stop or delete until the next deploy, update or start, or until a ready check more than two minutes later.

## Staying current

The board listens to the server's event stream, which announces every round of health checks and every change made through the API, and also refreshes every 20 seconds so that CLI changes show up. Ages keep counting on screen. If the server cannot be reached the board says so, keeps the last answer, and turns old checks into "stale". The dashboard's pause button also pauses the board.

`aether serve` checks every 60 seconds (`AETHER_OBSERVE_SECS`). With `0`, run `aether health collect` from a timer.

## Wall mode

**Wall mode** fills the screen for a shared monitor, enlarges the tiles, asks the browser to keep the display awake, and leaves with `Esc`. The address `/board?wall=1` opens it directly (browsers only enter true full screen after a click). `window` and `show` can be part of the same address, for example `/board?wall=1&window=6h&show=attention`.

## API and access

`GET /api/board?window=1h|6h|24h|7d|30d` returns the tiles. Viewer keys can read it; "Check now" (`POST /api/observe`) needs Operator or Admin. No spec content or secret values are included.
