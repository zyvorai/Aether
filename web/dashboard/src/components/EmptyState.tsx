import type { ReactNode } from 'react';
import { useTheme } from '../contexts/ThemeContext';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export default function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  const { theme } = useTheme();
  const light = theme === 'light';

  return (
    <div
      className={`relative flex flex-col items-center justify-center overflow-hidden rounded-2xl border px-6 py-16 text-center ${
        light
          ? 'border-slate-200 bg-white/80'
          : 'border-slate-800/70 bg-slate-950/35'
      }`}
    >
      <div
        className={`pointer-events-none absolute inset-0 ${
          light
            ? 'bg-[radial-gradient(circle_at_center,rgba(211,84,0,0.08),transparent_42%)]'
            : 'bg-[radial-gradient(circle_at_center,rgba(99,164,255,0.10),transparent_42%)]'
        }`}
      />
      <div
        className={`relative mb-4 rounded-2xl border p-4 shadow-inner ${
          light
            ? 'border-slate-200 bg-slate-50 text-slate-400'
            : 'border-slate-700/70 bg-slate-900/80 text-slate-500'
        }`}
      >
        {icon}
      </div>
      <h3 className={`relative text-lg font-semibold ${light ? 'text-slate-800' : 'text-slate-200'}`}>{title}</h3>
      {description && (
        <p className={`relative mt-2 max-w-md text-sm leading-relaxed ${light ? 'text-slate-600' : 'text-slate-500'}`}>
          {description}
        </p>
      )}
      {action && <div className="relative mt-5">{action}</div>}
    </div>
  );
}
