import type { ReactNode } from 'react';
import { Database, Radio, Server, Shield, Wifi, WifiOff } from 'lucide-react';
import type { PlatformInfo, SystemReadyStatus } from '../types/api';

interface PlatformStatusPanelProps {
  platform: PlatformInfo | null;
  ready: SystemReadyStatus | null;
  sseConnected: boolean;
  loading?: boolean;
}

type TileTone = 'ok' | 'warn' | 'bad' | 'idle';

function toneClasses(tone: TileTone): { ring: string; dot: string; text: string } {
  switch (tone) {
    case 'ok':
      return {
        ring: 'border-emerald-500/25 bg-emerald-500/10',
        dot: 'bg-emerald-400 platform-pulse',
        text: 'text-emerald-300',
      };
    case 'warn':
      return {
        ring: 'border-amber-500/25 bg-amber-500/10',
        dot: 'bg-amber-400',
        text: 'text-amber-300',
      };
    case 'bad':
      return {
        ring: 'border-red-500/25 bg-red-500/10',
        dot: 'bg-red-400',
        text: 'text-red-300',
      };
    default:
      return {
        ring: 'border-slate-600/30 bg-slate-800/40',
        dot: 'bg-slate-500',
        text: 'text-slate-400',
      };
  }
}

function StatusTile({
  icon,
  label,
  value,
  detail,
  tone,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail: string;
  tone: TileTone;
}) {
  const c = toneClasses(tone);
  return (
    <div
      className={`group relative overflow-hidden rounded-2xl border p-4 transition-all duration-300 hover:scale-[1.01] hover:shadow-lg ${c.ring}`}
    >
      <div className="pointer-events-none absolute -right-6 -top-6 h-24 w-24 rounded-full bg-white/5 blur-2xl transition-opacity group-hover:opacity-80" />
      <div className="flex items-start justify-between gap-3">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-black/20 text-aether">
          {icon}
        </div>
        <span className={`mt-1 h-2.5 w-2.5 shrink-0 rounded-full ${c.dot}`} title={value} />
      </div>
      <p className="mt-3 text-[10px] font-semibold uppercase tracking-[0.18em] text-slate-500">{label}</p>
      <p className={`mt-1 text-sm font-semibold ${c.text}`}>{value}</p>
      <p className="mt-1 text-xs leading-relaxed text-slate-500">{detail}</p>
    </div>
  );
}

export default function PlatformStatusPanel({ platform, ready, sseConnected, loading }: PlatformStatusPanelProps) {
  if (loading) {
    return (
      <div className="mb-8 grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-5">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="skeleton h-28 rounded-2xl" />
        ))}
      </div>
    );
  }

  if (!platform) {
    return (
      <section className="mb-8 rounded-2xl border border-slate-700/40 bg-slate-900/40 px-4 py-4 text-sm text-slate-400">
        <p className="font-medium text-slate-300">Platform status unavailable</p>
        <p className="mt-1 text-xs leading-relaxed">
          Could not load <code className="text-slate-300">/api/server</code>. Check that the API is running and your
          session is valid, then refresh the page.
        </p>
      </section>
    );
  }

  const postgresRequired = platform.workloadState.configured;
  const postgresOk = ready?.checks?.postgres?.ok ?? !postgresRequired;
  const redisRequired = platform.haSharedCache;
  const redisOk = ready?.checks?.redis?.ok ?? !redisRequired;
  const systemReady = ready?.ready ?? true;

  const stateTone: TileTone = !postgresRequired
    ? 'idle'
    : postgresOk
      ? 'ok'
      : 'bad';
  const cacheTone: TileTone = !redisRequired ? 'idle' : redisOk ? 'ok' : 'bad';
  const liveTone: TileTone = sseConnected ? 'ok' : 'warn';
  const apiTone: TileTone = systemReady ? 'ok' : 'bad';

  const stateValue =
    platform.workloadState.backend === 'postgresql'
      ? postgresOk
        ? 'PostgreSQL · synced'
        : 'PostgreSQL · unreachable'
      : 'Local JSON file';

  const stateDetail =
    platform.workloadState.backend === 'postgresql'
      ? `Poll ${platform.workloadState.pollSecs}s · ${platform.workloadState.env}`
      : 'Local JSON on this node';

  return (
    <section className="mb-8">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold tracking-tight text-white">Status</h2>
          <p className="mt-1 text-xs text-slate-500">
            v{platform.version}
            {platform.tls ? ' · TLS' : ''}
          </p>
        </div>
        <div
          className={`rounded-xl border px-3 py-1.5 text-xs font-medium ${
            systemReady
              ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-300'
              : 'border-red-500/30 bg-red-500/10 text-red-300'
          }`}
        >
          {systemReady ? 'Ready' : 'Degraded'}
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <StatusTile
          icon={<Server className="h-5 w-5" />}
          label="API"
          value={systemReady ? 'OK' : 'Not ready'}
          detail={platform.haMode.replace(/-/g, ' ')}
          tone={apiTone}
        />
        <StatusTile
          icon={<Database className="h-5 w-5" />}
          label="State"
          value={stateValue}
          detail={stateDetail}
          tone={stateTone}
        />
        <StatusTile
          icon={<Shield className="h-5 w-5" />}
          label="Sessions"
          value={redisRequired ? (redisOk ? 'Redis' : 'Down') : 'Local'}
          detail={redisRequired ? 'AETHER_REDIS_URL' : 'Optional for multi-node OIDC'}
          tone={cacheTone}
        />
        <StatusTile
          icon={sseConnected ? <Wifi className="h-5 w-5" /> : <WifiOff className="h-5 w-5" />}
          label="Live"
          value={sseConnected ? 'SSE' : 'SSE off'}
          detail="Event stream"
          tone={liveTone}
        />
        <StatusTile
          icon={<Radio className="h-5 w-5" />}
          label="Auth"
          value={
            platform.oidc.enabled
              ? 'OIDC'
              : platform.safety.mutation_confirm_required
                ? 'Strict'
                : 'Open'
          }
          detail={
            platform.oidc.enabled
              ? 'RBAC + IdP'
              : platform.safety.mutation_confirm_required
                ? platform.safety.mutation_confirm_header
                : 'Set API key or OIDC'
          }
          tone={platform.oidc.enabled || platform.safety.mutation_confirm_required ? 'ok' : 'warn'}
        />
      </div>
    </section>
  );
}
