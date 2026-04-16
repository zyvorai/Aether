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

export function getRuntimeColor(runtime: string): string {
  const r = runtime.toLowerCase();
  if (r.includes('podman')) return 'text-blue-400';
  if (r.includes('kubevirt') || r.includes('virt')) return 'text-purple-400';
  if (r.includes('kube') || r.includes('k8s')) return 'text-emerald-400';
  if (r.includes('metal')) return 'text-red-400';
  if (r.includes('docker')) return 'text-cyan-400';
  return 'text-zinc-400';
}

export function getRuntimeBg(runtime: string): string {
  const r = runtime.toLowerCase();
  if (r.includes('podman')) return 'bg-blue-500/10 text-blue-400 border-blue-500/20';
  if (r.includes('kubevirt') || r.includes('virt')) return 'bg-purple-500/10 text-purple-400 border-purple-500/20';
  if (r.includes('kube') || r.includes('k8s')) return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20';
  if (r.includes('metal')) return 'bg-red-500/10 text-red-400 border-red-500/20';
  if (r.includes('docker')) return 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20';
  return 'bg-zinc-500/10 text-zinc-400 border-zinc-500/20';
}

export function getSeverityColor(severity: string): string {
  const s = severity.toUpperCase();
  if (s === 'CRITICAL' || s === 'ERROR') return 'bg-red-500/10 text-red-400 border-red-500/20';
  if (s === 'WARNING' || s === 'WARN') return 'bg-amber-500/10 text-amber-400 border-amber-500/20';
  if (s === 'INFO') return 'bg-blue-500/10 text-blue-400 border-blue-500/20';
  return 'bg-zinc-500/10 text-zinc-400 border-zinc-500/20';
}

export function getHealthColor(health: string): string {
  const h = health.toLowerCase();
  if (h === 'healthy') return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20';
  if (h === 'degraded') return 'bg-amber-500/10 text-amber-400 border-amber-500/20';
  if (h === 'unhealthy') return 'bg-red-500/10 text-red-400 border-red-500/20';
  return 'bg-zinc-500/10 text-zinc-400 border-zinc-500/20';
}

export function getBarColor(percent: number): string {
  if (percent > 85) return 'bg-gradient-to-r from-red-600 to-red-500';
  if (percent > 60) return 'bg-gradient-to-r from-amber-600 to-amber-500';
  return 'bg-gradient-to-r from-emerald-600 to-emerald-500';
}
