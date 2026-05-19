interface PageLoadingProps {
  rows?: number;
  variant?: 'skeleton' | 'spinner';
}

export default function PageLoading({ rows = 4, variant = 'skeleton' }: PageLoadingProps) {
  if (variant === 'spinner') {
    return (
      <div className="flex items-center justify-center py-16">
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-aether border-t-transparent" />
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
