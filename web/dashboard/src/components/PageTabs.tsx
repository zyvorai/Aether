import type { ReactNode } from 'react';

export interface PageTab<T extends string> {
  id: T;
  label: string;
  icon?: ReactNode;
}

interface PageTabsProps<T extends string> {
  tabs: PageTab<T>[];
  active: T;
  onChange: (id: T) => void;
  className?: string;
}

export default function PageTabs<T extends string>({
  tabs,
  active,
  onChange,
  className = '',
}: PageTabsProps<T>) {
  return (
    <div className={`flex flex-wrap gap-1 border-b border-slate-800 pb-1 mb-6 ${className}`}>
      {tabs.map((tab) => {
        const isActive = active === tab.id;
        return (
          <button
            key={tab.id}
            type="button"
            onClick={() => onChange(tab.id)}
            aria-selected={isActive}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-t-xl text-sm font-medium transition-all ${
              isActive
                ? 'bg-slate-900/80 text-white border-b-2 border-aether'
                : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/40'
            }`}
          >
            {tab.icon}
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
