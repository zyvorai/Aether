// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { Activity } from 'lucide-react';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';

interface PlatformHealthChipProps {
  sseConnected: boolean;
}

export default function PlatformHealthChip({ sseConnected }: PlatformHealthChipProps) {
  const { capabilities, ready, loading } = useServerCapabilities();

  if (loading && !capabilities) {
    return (
      <span className="hidden lg:inline-flex h-9 w-24 skeleton rounded-xl" title="Loading platform status" />
    );
  }

  const platform = capabilities?.platform;
  const systemReady = ready?.ready ?? true;
  const postgresOk = ready?.checks?.postgres?.ok ?? true;
  const redisOk = ready?.checks?.redis?.ok ?? true;
  const allOk = systemReady && postgresOk && redisOk && sseConnected;

  const label = !platform
    ? 'Platform'
    : platform.workloadState.backend === 'postgresql'
      ? 'HA · Postgres'
      : platform.haSharedCache
        ? 'HA · Redis'
        : 'Single node';

  const title = [
    platform ? `Aether v${platform.version}` : 'Aether',
    systemReady ? 'API ready' : 'API not ready',
    platform?.workloadState.backend === 'postgresql'
      ? postgresOk
        ? 'PostgreSQL connected'
        : 'PostgreSQL unreachable'
      : 'Local JSON state',
    platform?.haSharedCache ? (redisOk ? 'Redis connected' : 'Redis unreachable') : 'In-process OIDC cache',
    sseConnected ? 'Live SSE connected' : 'Live SSE disconnected',
  ].join(' · ');

  return (
    <span
      className={`hidden lg:inline-flex items-center gap-1.5 rounded-xl border px-2 py-1.5 text-xs font-medium transition-colors xl:gap-2 xl:px-3 xl:py-2 ${
        allOk
          ? 'border-success/25 bg-success/10 text-success'
          : 'border-warning/25 bg-warning/10 text-warning'
      }`}
      title={title}
    >
      <Activity className={`h-3.5 w-3.5 shrink-0 ${allOk ? 'text-success' : 'text-warning'}`} />
      <span className="hidden whitespace-nowrap xl:inline">{label}</span>
      <span
        data-testid="navbar-sse-status-desktop"
        className={`h-2 w-2 shrink-0 rounded-full ${allOk ? 'bg-success platform-pulse' : 'bg-warning'}`}
      />
    </span>
  );
}
