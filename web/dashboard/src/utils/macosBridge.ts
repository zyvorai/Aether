// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

type TauriInvoke = (cmd: string, args?: object) => Promise<unknown>;

function tauriInvoke(): TauriInvoke | undefined {
  return (window as Window & { __TAURI__?: { core?: { invoke?: TauriInvoke } } }).__TAURI__?.core
    ?.invoke;
}

export async function syncMacOSTray(
  fleetHealthPct: number,
  issues: number,
  sparkline?: string,
): Promise<void> {
  const invoke = tauriInvoke();
  if (!invoke) return;
  await invoke('tray_health_update', {
    fleet_health_pct: fleetHealthPct,
    issues,
    sparkline: sparkline ?? null,
  }).catch(() => {});
  if (sparkline) {
    await invoke('tray_sparkline_update', { sparkline, health: fleetHealthPct }).catch(() => {});
  }
}

export async function syncMacOSDockBadge(issueCount: number): Promise<void> {
  const invoke = tauriInvoke();
  if (!invoke) return;
  await invoke('dock_badge_update', { issue_count: issueCount }).catch(() => {});
}

export async function showMacOSNotification(title: string, body: string): Promise<void> {
  const invoke = tauriInvoke();
  if (!invoke) return;
  await invoke('native_notification_show', { title, body }).catch(() => {});
}

export async function syncMacOSLiveActivity(
  workload: string,
  progressPct: number,
  detail: string,
): Promise<void> {
  const invoke = tauriInvoke();
  if (!invoke) return;
  await invoke('live_activity_update', {
    workload,
    progress_pct: progressPct,
    detail,
  }).catch(() => {});
}

export function isMacOSShell(): boolean {
  return Boolean(tauriInvoke());
}

/** Listen for native shell navigation (tray menu, deep links). */
export function subscribeMacOSNavigate(onNavigate: (path: string) => void): () => void {
  const handler = (event: Event) => {
    const detail = (event as CustomEvent<{ path?: string }>).detail;
    if (detail?.path) onNavigate(detail.path);
  };
  window.addEventListener('aether-navigate', handler);
  return () => window.removeEventListener('aether-navigate', handler);
}
