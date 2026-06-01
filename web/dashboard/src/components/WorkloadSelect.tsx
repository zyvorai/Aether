// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { WorkloadResponse } from '../types/api';

interface WorkloadSelectProps {
  workloads: WorkloadResponse[] | string[];
  value: string;
  onChange: (name: string) => void;
  placeholder?: string;
  className?: string;
  allowEmpty?: boolean;
}

export default function WorkloadSelect({
  workloads,
  value,
  onChange,
  placeholder = 'Select workload…',
  className = '',
  allowEmpty = true,
}: WorkloadSelectProps) {
  const names = workloads.map((w) => (typeof w === 'string' ? w : w.name));

  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className={`glass-select ${className}`}
    >
      {allowEmpty && <option value="">{placeholder}</option>}
      {names.map((name) => (
        <option key={name} value={name}>
          {name}
        </option>
      ))}
    </select>
  );
}
