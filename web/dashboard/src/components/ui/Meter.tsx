// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
      className={`inline-block h-1 w-9 rounded-full ${className}`}
      style={{ background: 'var(--rule)', verticalAlign: '2px', marginRight: 7 }}
    >
      <span
        className="block h-1 rounded-full"
        style={{ width: `${Math.min(100, Math.max(0, value))}%`, background: tone }}
      />
    </span>
  );
}
