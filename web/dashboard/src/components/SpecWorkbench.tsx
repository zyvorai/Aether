// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import YamlInput from './YamlInput';

interface SpecWorkbenchProps {
  title: string;
  description?: string;
  buttonText: string;
  onSubmit: (yaml: string) => void;
  loading?: boolean;
  placeholder?: string;
  submitTestId?: string;
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
  submitTestId,
  result,
  sidePanel,
}: SpecWorkbenchProps) {

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div className="dash-card">
        <h2 className={`text-lg font-semibold mb-1 ${'text-ink'}`}>{title}</h2>
        {description && (
          <p className={`text-sm mb-4 ${'text-ink-3'}`}>{description}</p>
        )}
        <YamlInput
          buttonText={buttonText}
          onSubmit={onSubmit}
          loading={loading}
          placeholder={placeholder}
          submitTestId={submitTestId}
        />
        {sidePanel}
      </div>
      <div className="dash-card min-h-[12rem]">
        <h3 className={`text-sm font-medium uppercase tracking-wider mb-4 ${'text-ink-2'}`}>
          Results
        </h3>
        {result ?? (
          <p className={`text-sm ${'text-ink-3'}`}>
            Submit a spec to see validation and analysis output here.
          </p>
        )}
      </div>
    </div>
  );
}
