import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
}

export default function EmptyState({ icon, title, description }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="text-zinc-600 mb-4">{icon}</div>
      <h3 className="text-lg font-medium text-zinc-400">{title}</h3>
      {description && (
        <p className="text-sm text-zinc-500 mt-1 max-w-md">{description}</p>
      )}
    </div>
  );
}
