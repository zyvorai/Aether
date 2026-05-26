interface PageLoadingProps {
  rows?: number;
  variant?: 'skeleton' | 'spinner';
  label?: string;
  className?: string;
}

export default function PageLoading({ rows = 4, variant = 'skeleton', label, className }: PageLoadingProps) {

  if (variant === 'spinner' || label) {
    return (
      <div className={`flex flex-col items-center justify-center gap-3 ${className ?? 'py-16'}`}>
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-aether border-t-transparent" />
        {label ? <p className={`text-sm ${'text-slate-500'}`}>{label}</p> : null}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="skeleton h-20 rounded-xl" />
      ))}
    </div>
  );
}
