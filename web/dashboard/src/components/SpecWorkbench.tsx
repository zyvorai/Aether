import type { ReactNode } from 'react';
import YamlInput from './YamlInput';
import { useTheme } from '../contexts/ThemeContext';

interface SpecWorkbenchProps {
  title: string;
  description?: string;
  buttonText: string;
  onSubmit: (yaml: string) => void;
  loading?: boolean;
  placeholder?: string;
  result?: ReactNode;
  sidePanel?: ReactNode;
}

export default function SpecWorkbench({
  title,
  description,
  buttonText,
  onSubmit,
  loading = false,
  placeholder,
  result,
  sidePanel,
}: SpecWorkbenchProps) {
  const { theme } = useTheme();
  const light = theme === 'light';

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div className="dash-card">
        <h2 className={`text-lg font-semibold mb-1 ${light ? 'text-slate-900' : 'text-slate-100'}`}>{title}</h2>
        {description && (
          <p className={`text-sm mb-4 ${light ? 'text-slate-600' : 'text-slate-500'}`}>{description}</p>
        )}
        <YamlInput
          buttonText={buttonText}
          onSubmit={onSubmit}
          loading={loading}
          placeholder={placeholder}
        />
        {sidePanel}
      </div>
      <div className="dash-card min-h-[12rem]">
        <h3 className={`text-sm font-medium uppercase tracking-wider mb-4 ${light ? 'text-slate-500' : 'text-slate-400'}`}>
          Results
        </h3>
        {result ?? (
          <p className={`text-sm ${light ? 'text-slate-600' : 'text-slate-500'}`}>
            Submit a spec to see validation and analysis output here.
          </p>
        )}
      </div>
    </div>
  );
}
