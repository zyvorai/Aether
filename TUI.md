# 🖥️ Orchestr8 TUI - Terminal User Interface

The Orchestr8 TUI provides an interactive dashboard for managing workloads across multiple runtimes.

## Features

- 📊 **Live Dashboard** - Real-time workload status
- 🔄 **Auto-refresh** - Updates every 5 seconds
- 🔍 **Search/Filter** - Filter workloads by name or runtime (`/` to search)
- 📝 **Log Viewer** - View logs without leaving the interface
- 📋 **Resource Panel** - CPU, memory, storage, GPU details in the detail view
- 🎨 **Color-coded** - Visual status indicators
- ⌨️ **Keyboard Navigation** - Efficient workflow
- 🐳☸️ **Multi-Runtime** - Podman + Kubernetes + KubeVirt + Metal3 support

---

## Quick Start

Launch the TUI:

```bash
orchestr8 tui
```

That's it! The dashboard will load automatically.

---

## Dashboard Screen

### Layout

```
┌────────────────────────────────────────────┐
│         ORCHESTR8 - Universal Runtime      │
│      Workloads: 3 | Last refresh: 2s ago   │
└────────────────────────────────────────────┘
┌─ Workloads ───────┬─ Detail ──────────────┐
│                    │  Name:     my-app     │
│▸ my-app  🐳 podman│  Runtime:  🐳 Podman  │
│  web-app ☸️ kube  │  Instance: abc123...  │
│  db      🐳 podman│  ─── Resources ───    │
│                    │  CPU:      2          │
│                    │  Memory:   4Gi        │
│                    │  Storage:  20Gi       │
│                    │  Health:   ████████░░ │
└────────────────────┴───────────────────────┘
┌────────────────────────────────────────────┐
│ [/] search  ↑↓ Select │ Enter Logs │ q Quit│
└────────────────────────────────────────────┘
```

### Workload Information

Each workload displays:

- **Name** - Workload identifier
- **Runtime Icon** - 🐳 Podman, ☸️ Kubernetes, 🖥️ KubeVirt, 🖧 Metal3
- **Status Indicator**:
  - **●** (filled) - Ready
  - **○** (hollow) - Not ready
- **State** - running, pending, stopped, failed
- **Restart Count** - If > 0
- **Message** - Error or status details

### Color Coding

| Element | Color | Meaning |
|---------|-------|---------|
| ● Green | Running & ready |
| ○ Red | Not ready |
| Yellow | Pending |
| Gray | Stopped |
| Red | Failed |

### Runtime Badges

| Badge | Runtime |
|-------|---------|
| 🐳 podman | Podman container |
| ☸️ kubernetes | Kubernetes pod |
| 🖥️ kubevirt | KubeVirt VM |
| 🖧 metal3 | Bare metal |

---

## Keyboard Controls

### Dashboard

| Key | Action |
|-----|--------|
| `↑` or `k` | Select previous workload |
| `↓` or `j` | Select next workload |
| `Enter` or `l` | View logs for selected workload |
| `/` | Open search/filter mode |
| `Esc` | Clear search filter |
| `g` | Jump to first workload |
| `G` | Jump to last workload |
| `r` or `R` | Force refresh status |
| `q` or `Q` | Quit application |

### Search/Filter Mode

When search mode is active (press `/`):

| Key | Action |
|-----|--------|
| Any character | Append to search query |
| `Backspace` | Remove last character |
| `Enter` | Confirm filter (keep filter, exit input mode) |
| `Esc` | Cancel and clear filter |

Search matches against workload **name** and **runtime** (case-insensitive).

### Log Viewer

| Key | Action |
|-----|--------|
| `Esc` | Return to dashboard |
| `q` or `Q` | Quit application |

---

## Log Viewer Screen

### Layout

```
┌────────────────────────────────────────────┐
│            Logs: my-app                     │
└────────────────────────────────────────────┘
┌─ Output ───────────────────────────────────┐
│ 2024-02-06 12:00:01 Starting server...     │
│ 2024-02-06 12:00:02 Server listening on 80 │
│ 2024-02-06 12:00:05 Request GET /          │
│ 2024-02-06 12:00:06 Response 200           │
│ 2024-02-06 12:00:10 ERROR: Database conn   │
│ 2024-02-06 12:00:11 WARN: Retrying...      │
│                                             │
│                                             │
└────────────────────────────────────────────┘
┌────────────────────────────────────────────┐
│           Esc Back | q Quit                 │
└────────────────────────────────────────────┘
```

### Log Colors

Logs are automatically color-coded:

| Keyword | Color | Purpose |
|---------|-------|---------|
| ERROR, error | Red | Critical errors |
| WARN, warn | Yellow | Warnings |
| INFO, info | Green | Information |
| Default | White | Normal output |

### Features

- **Auto-scrolling** - Shows latest logs
- **Fits to screen** - Displays maximum visible lines
- **Real-time** - Updates when entering log view

---

## Auto-Refresh

The dashboard automatically refreshes every **5 seconds** to keep status current.

### What Gets Updated

- Workload states (running, stopped, etc.)
- Readiness status
- Restart counts
- Status messages

### Manual Refresh

Press `r` to force an immediate refresh.

---

## Example Workflow

### 1. Launch TUI

```bash
orchestr8 tui
```

### 2. Navigate Dashboard

- Use `↓` to browse workloads
- See real-time status for all deployments
- Check which runtime each workload uses

### 3. View Logs

- Select a workload with `↓`/`↑`
- Press `Enter` to view logs
- Read color-coded output
- Press `Esc` to return to dashboard

### 4. Monitor Status

- Watch auto-refresh every 5s
- See state changes in real-time
- Check restart counts

### 5. Exit

- Press `q` from any screen

---

## Use Cases

### 1. Multi-Runtime Monitoring

Monitor workloads across Podman and Kubernetes simultaneously:

```bash
# Deploy to both runtimes
orchestr8 run --spec app1.yaml --runtime podman
orchestr8 run --spec app2.yaml --runtime kube

# Monitor both
orchestr8 tui
```

View both at once in the dashboard.

### 2. Debugging

Quick log access without typing commands:

```bash
orchestr8 tui
# Navigate to failed workload
# Press Enter
# Read error logs
# Press Esc to go back
```

### 3. Status Checking

Visual health check for all deployments:

```bash
orchestr8 tui
# See all statuses at a glance
# Green ● = healthy
# Red ○ = unhealthy
```

### 4. Development Monitoring

Keep TUI open while developing:

```bash
# Terminal 1
orchestr8 tui

# Terminal 2
vim src/main.rs
orchestr8 run --runtime podman

# Watch status change in TUI automatically
```

---

## Empty State

If no workloads are running:

```
┌─ Workloads ────────────────────────────────┐
│                                             │
│           No workloads running              │
│                                             │
│     Run 'orchestr8 run' to deploy           │
│                                             │
└────────────────────────────────────────────┘
```

---

## Troubleshooting

### TUI Won't Launch

**Issue:** `orchestr8 tui` fails

**Solution:**
```bash
# Check terminal support
echo $TERM

# Try with explicit terminal
TERM=xterm-256color orchestr8 tui
```

### Colors Not Showing

**Issue:** No colors in TUI

**Solution:**
- Use a modern terminal (kitty, alacritty, iTerm2, Windows Terminal)
- Enable true color support
- Set `TERM=xterm-256color`

### Workloads Not Appearing

**Issue:** Dashboard is empty but workloads exist

**Solution:**
```bash
# Verify state file
cat ~/.orchestr8/state.json

# Check CLI list
orchestr8 list

# Force refresh in TUI (press 'r')
```

### Status Not Updating

**Issue:** Status shows "Loading..." forever

**Possible causes:**
- Kubernetes cluster unreachable
- Podman not running
- Permissions issue

**Solution:**
```bash
# Test Kubernetes
kubectl get pods

# Test Podman
podman ps

# Check logs
orchestr8 -v list
```

---

## Advanced Tips

### 1. Quick Navigation

Use vim-style keys for fast navigation:
- `j` = down
- `k` = up
- `l` = open logs

### 2. Monitoring Loop

Keep TUI open in a dedicated terminal or tmux pane:

```bash
# In tmux
tmux new -s orchestr8
orchestr8 tui

# Detach: Ctrl+B, D
# Reattach: tmux attach -t orchestr8
```

### 3. Combine with Watch

Monitor specific workload in parallel:

```bash
# Terminal 1: TUI
orchestr8 tui

# Terminal 2: Watch specific pod
watch -n1 kubectl get pod my-app
```

### 4. CI/CD Integration

While TUI is for interactive use, use CLI in scripts:

```bash
# Deploy
orchestr8 run --spec app.yaml

# Monitor interactively
orchestr8 tui
```

---

## Comparison: TUI vs CLI

| Feature | TUI | CLI |
|---------|-----|-----|
| **Visual** | ✅ Rich colors & layout | ⚠️ Plain text |
| **Multi-workload** | ✅ All at once | ❌ One at a time |
| **Logs** | ✅ Built-in viewer | ⚠️ Separate command |
| **Auto-refresh** | ✅ Every 5s | ❌ Manual |
| **Scripting** | ❌ Interactive only | ✅ Scriptable |
| **Remote** | ⚠️ Needs terminal | ✅ SSH-friendly |

**Recommendation:**
- **TUI** for local development and monitoring
- **CLI** for automation and scripting

---

## Keyboard Reference Card

```
┌─────────────────────────────────────┐
│      ORCHESTR8 TUI SHORTCUTS        │
├─────────────────────────────────────┤
│  NAVIGATION                         │
│  ↑/k ............... Select previous│
│  ↓/j ............... Select next    │
│  g ................. Jump to first  │
│  G ................. Jump to last   │
│  Enter/l ........... View logs      │
│  Esc ............... Clear/Back     │
│                                     │
│  SEARCH                             │
│  / ................. Open filter    │
│  Enter ............. Confirm filter │
│  Esc ............... Clear filter   │
│                                     │
│  ACTIONS                            │
│  r ................. Refresh now    │
│  q ................. Quit           │
└─────────────────────────────────────┘
```

---

## Future Enhancements

Planned features:

- **Horizontal scrolling** for long names
- **Sorting** by name, runtime, status
- **Filtering** by runtime or state
- **Delete from TUI** (press `d`)
- **Restart from TUI** (press `R`)
- **Detailed view** (press `i` for info)
- **Resource graphs** (CPU, memory usage)
- **Multi-panel layout** (dashboard + logs)
- **Search/filter** (press `/`)

---

## Architecture

### TUI Components

```
src/ui/
├── mod.rs          # Module exports
├── app.rs          # Application state
├── dashboard.rs    # Dashboard screen
├── logs.rs         # Log viewer screen
├── components.rs   # Reusable widgets
└── events.rs       # Keyboard handling
```

### State Flow

```
App State
   ├─ Workloads (from state.json)
   ├─ Current Screen (Dashboard/Logs)
   ├─ Selected Index
   ├─ Search Filter (name/runtime query)
   ├─ Search Active (input mode flag)
   ├─ Logs Buffer
   └─ Status Messages
      ↓
   Render Loop
      ↓
   Event Handler
      ↓
   Update State
      ↓
   Re-render
```

---

## Examples

### Monitor Production

```bash
# SSH to production
ssh prod-server

# Launch TUI
orchestr8 tui

# Watch all services
# Press Enter on failing service
# Read error logs
# Press Esc, fix issue
```

### Development Workflow

```bash
# Start TUI in tmux
tmux new -s dev
orchestr8 tui

# Split pane (Ctrl+B, ")
# Bottom pane: edit code
vim src/app.rs

# Deploy changes
orchestr8 run --runtime podman

# Watch status update in top pane automatically
```

### Multi-Cluster Monitoring

```bash
# Context 1: Dev cluster
kubectl config use-context dev
ORCHESTR8_NAMESPACE=dev orchestr8 tui &

# Context 2: Prod cluster
kubectl config use-context prod
ORCHESTR8_NAMESPACE=prod orchestr8 tui
```

---

## Performance

- **Startup:** < 200ms
- **Refresh:** < 500ms for 10 workloads
- **Memory:** ~5MB
- **CPU:** < 1% idle, ~5% during refresh

---

## Summary

The Orchestr8 TUI provides:

✅ **Visual monitoring** of all workloads
✅ **Multi-runtime** support (Podman + Kubernetes)
✅ **Real-time updates** (auto-refresh)
✅ **Integrated log viewer**
✅ **Keyboard-driven** workflow
✅ **Search/filter** workloads by name or runtime
✅ **Resource details** (CPU, memory, storage, GPU) in detail panel
✅ **Color-coded** status indicators

Perfect for:
- 👨‍💻 Development monitoring
- 🔍 Debugging sessions
- 📊 Production health checks
- 🚀 Demo presentations

**Try it now:**
```bash
orchestr8 tui
```
