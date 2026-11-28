// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

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
            aria-pressed={isActive}
            className={`rounded-full px-3 py-1.5 text-xs font-medium transition-colors ${
              isActive
                ? 'border border-primary/40 bg-primary/10 text-primary'
                : 'border border-border bg-surface-elevated text-muted hover:text-foreground'
            }`}
          >
            {chip.label}
            {chip.count !== undefined ? <span className="ml-1 tabular-nums">{chip.count}</span> : null}
          </button>
        );
      })}
    </div>
  );
}
