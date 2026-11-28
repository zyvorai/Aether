// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

interface MeterProps {
  value: number;
  warnAt?: number;
  dangerAt?: number;
  className?: string;
}

export default function Meter({ value, warnAt = 75, dangerAt = 90, className = '' }: MeterProps) {
  const tone = value >= dangerAt ? 'var(--danger)' : value >= warnAt ? 'var(--warning)' : 'var(--muted-foreground)';
  return (
    <span
      className={`inline-block h-1.5 w-10 overflow-hidden rounded-full ${className}`}
      style={{ background: 'var(--track)', verticalAlign: '1px', marginRight: 7 }}
    >
      <span
        className="block h-1.5 rounded-full transition-[width] duration-500"
        style={{ width: `${Math.min(100, Math.max(0, value))}%`, background: tone }}
      />
    </span>
  );
}
