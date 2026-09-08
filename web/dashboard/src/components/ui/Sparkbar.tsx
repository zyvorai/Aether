// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

interface SparkbarProps {
  window: number[];
  tone?: string;
  className?: string;
}

export default function Sparkbar({ window: win, tone = 'var(--danger)', className = '' }: SparkbarProps) {
  return (
    <span className={`inline-flex items-end gap-[2px] ${className}`} style={{ verticalAlign: '-1px' }}>
      {win.map((v, i) => (
        <span
          key={i}
          className="block w-[2px] rounded-[1px]"
          style={{
            height: v ? 10 : 3,
            background: v ? tone : 'var(--rule-strong)',
          }}
        />
      ))}
    </span>
  );
}
