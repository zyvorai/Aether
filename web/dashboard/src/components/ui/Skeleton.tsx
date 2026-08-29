import { cn } from '../../lib/cn';

export function SkeletonLine({ className }: { className?: string }) {
  return <div className={cn('animate-shimmer rounded-[var(--radius-sm)] h-4', className)} aria-hidden />;
}
export function SkeletonCard() {
  return <div className="glass rounded-[var(--radius-liquid)] p-5 space-y-4" aria-hidden><div className="space-y-2"><SkeletonLine className="w-1/2" /><SkeletonLine className="w-1/3 h-3" /></div><div className="flex gap-2"><SkeletonLine className="h-9 flex-1" /><SkeletonLine className="h-9 flex-1" /></div></div>;
}
export function SkeletonTable({ rows = 5, columns = 4 }: { rows?: number; columns?: number }) {
  return <div className="glass rounded-[var(--radius-liquid)] overflow-hidden" aria-hidden><div className="flex gap-4 px-4 py-3 border-b border-border">{Array.from({ length: columns }, (_, i) => <SkeletonLine key={i} className="h-3 flex-1" />)}</div>{Array.from({ length: rows }, (_, row) => <div key={row} className="flex gap-4 px-4 py-3.5 border-b border-border last:border-0">{Array.from({ length: columns }, (_, col) => <SkeletonLine key={col} className="flex-1" />)}</div>)}</div>;
}
export function SkeletonHero() {
  return <div className="tahoe-hero p-5 lg:p-6 space-y-3" aria-hidden><div className="flex items-start gap-4"><SkeletonLine className="w-[3.25rem] h-[3.25rem] rounded-[var(--radius-liquid)] shrink-0" /><div className="space-y-2 flex-1 max-w-md pt-1"><SkeletonLine className="w-2/3 h-6" /><SkeletonLine className="w-full h-4" /></div></div></div>;
}
export function SkeletonText({ lines = 4 }: { lines?: number }) {
  return <div className="space-y-3" aria-hidden>{Array.from({ length: lines }, (_, i) => <SkeletonLine key={i} className={i === lines - 1 ? 'w-2/3' : 'w-full'} />)}</div>;
}
