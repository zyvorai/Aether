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
      className={`rounded-xl border border-slate-700/80 bg-slate-950/60 px-3 py-2 text-sm text-slate-100 outline-none focus:border-aether/50 focus-visible:ring-2 focus-visible:ring-aether/30 ${className}`}
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
