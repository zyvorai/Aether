// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

export function formatTimestamp(ts: string): string {
  return (ts || '').substring(0, 19).replace('T', ' ');
}

export function formatRelativeTime(ts: string): string {
  const now = Date.now();
  const then = new Date(ts).getTime();
  const diff = Math.floor((now - then) / 1000);

  if (diff < 60) return 'just now';
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function formatDuration(secs: number): string {
  if (secs < 60) return `${secs.toFixed(0)}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${Math.floor(secs % 60)}s`;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}

export function formatPercent(value: number, decimals = 1): string {
  return `${(value * 100).toFixed(decimals)}%`;
}

export function formatUSD(amount: number): string {
  return `$${amount.toFixed(2)}`;
}

/*
  Runtime identity colors — iPhone 17 categorical palette (see theme.css).
  Each runtime gets a distinct non-brand, non-semantic tone so runtimes stay
  visually distinguishable at a glance without colliding with severity
  red/amber/green.
*/
export function getRuntimeColor(runtime: string): string {
  const r = runtime.toLowerCase();
  if (r.includes('podman')) return 'text-deepblue';
  if (r.includes('kubevirt') || r.includes('virt')) return 'text-lavender';
  if (r.includes('kube') || r.includes('k8s')) return 'text-sage';
  if (r.includes('docker')) return 'text-mistblue';
  return 'text-subtle';
}

export function getRuntimeBg(runtime: string): string {
  const r = runtime.toLowerCase();
  if (r.includes('podman')) return 'bg-deepblue/10 text-deepblue border-deepblue/20';
  if (r.includes('kubevirt') || r.includes('virt')) return 'bg-lavender/10 text-lavender border-lavender/20';
  if (r.includes('kube') || r.includes('k8s')) return 'bg-sage/10 text-sage border-sage/20';
  if (r.includes('docker')) return 'bg-mistblue/10 text-mistblue border-mistblue/20';
  return 'bg-hover text-subtle border-rule';
}

export function getSeverityColor(severity: string): string {
  const s = severity.toUpperCase();
  if (s === 'CRITICAL' || s === 'ERROR') return 'bg-danger/10 text-danger border-danger/20';
  if (s === 'WARNING' || s === 'WARN') return 'bg-warning/10 text-warning border-warning/20';
  if (s === 'INFO') return 'bg-primary/10 text-primary border-primary/20';
  return 'bg-slate-500/10 text-slate-400 border-slate-500/20';
}

export function getHealthColor(health: string): string {
  const h = health.toLowerCase();
  if (h === 'healthy') return 'bg-success/10 text-success border-success/20';
  if (h === 'degraded') return 'bg-warning/10 text-warning border-warning/20';
  if (h === 'unhealthy') return 'bg-danger/10 text-danger border-danger/20';
  return 'bg-slate-500/10 text-slate-400 border-slate-500/20';
}

export function getBarColor(percent: number): string {
  if (percent > 85) return 'bg-gradient-to-r from-red-600 to-red-500';
  if (percent > 60) return 'bg-gradient-to-r from-amber-600 to-amber-500';
  return 'bg-gradient-to-r from-emerald-600 to-emerald-500';
}
