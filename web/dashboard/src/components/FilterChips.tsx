// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

export interface FilterChip {
  id: string;
  label: string;
  count?: number;
}

interface FilterChipsProps {
  chips: FilterChip[];
  active: string;
  onSelect: (id: string) => void;
  testId?: string;
  className?: string;
}

export default function FilterChips({ chips, active, onSelect, testId, className }: FilterChipsProps) {
  return (
    <div className={`flex flex-wrap gap-1.5 ${className ?? ''}`} data-testid={testId}>
      {chips.map((chip) => {
        const isActive = active === chip.id;
        return (
          <button
            key={chip.id}
            type="button"
            onClick={() => onSelect(chip.id)}
            className={`rounded-full px-2.5 py-1 text-[11px] transition-colors ${
              isActive
                ? 'border border-aether/40 bg-aether/20 text-aether'
                : 'glass-inset-surface border glass-divider text-slate-400 hover:text-slate-200'
            }`}
          >
            {chip.label}
            {chip.count !== undefined ? <span className="ml-1 opacity-70">{chip.count}</span> : null}
          </button>
        );
      })}
    </div>
  );
}
