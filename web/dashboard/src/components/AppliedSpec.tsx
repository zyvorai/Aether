// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';
import { loadAppliedSpec, type AppliedSpecResult } from '../utils/appliedSpec';
import { formatTimestamp } from '../utils/formatters';
import CodeBlock from './CodeBlock';

export default function AppliedSpec({ name }: { name: string }) {
  const [result, setResult] = useState<AppliedSpecResult | null>(null);

  useEffect(() => {
    let current = true;
    setResult(null);
    void loadAppliedSpec(name).then((loaded) => {
      if (current) setResult(loaded);
    });
    return () => {
      current = false;
    };
  }, [name]);

  if (!result) return <p className="text-subtle">Loading the applied spec...</p>;
  if (result.state === 'failed') return <p className="text-danger">The applied spec could not be loaded: {result.detail}</p>;
  if (result.state === 'none') {
    return <p className="text-subtle">No applied revision is recorded for this workload. Revisions are recorded on deploy and update.</p>;
  }
  const { revision } = result;
  return (
    <div className="space-y-2" data-testid="workload-applied-spec">
      <p className="text-xs text-muted">
        Revision {revision.id}, applied {formatTimestamp(revision.at)} by {revision.actor}. Secret values are hidden.
      </p>
      <CodeBlock title="Applied spec">{revision.spec_yaml}</CodeBlock>
    </div>
  );
}
