// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { Link } from 'react-router';
import { Lock } from 'lucide-react';
import { viewToPath } from '../utils/dashboardRoutes';
import GlassSection from './GlassSection';

interface ConfidentialWorkloadPanelProps {
  workloadName: string;
  runtime?: string;
}

/** Workload confidential panel — Ragnarok is a separate Zyvor product. */
export default function ConfidentialWorkloadPanel({
  workloadName,
}: ConfidentialWorkloadPanelProps) {
  return (
    <GlassSection
      title="Confidential computing"
      subtitle={`Workload ${workloadName}`}
      data-testid="confidential-workload-unavailable"
    >
      <div className="flex items-start gap-3 text-sm text-muted">
        <Lock className="h-5 w-5 text-primary shrink-0 mt-0.5" aria-hidden />
        <div className="space-y-2">
          <p>
            Attestation, measured images, and confidential migration are provided by{' '}
            <strong className="text-ink">Ragnarok</strong>, a separate Zyvor product — not included
            in Apache-2.0 Aether-core.
          </p>
          <p>
            <Link to={viewToPath('confidential')} className="text-primary hover:underline">
              Learn more
            </Link>
            {' · '}
            <a href="mailto:licensing@zyvor.dev" className="text-primary hover:underline">
              licensing@zyvor.dev
            </a>
          </p>
        </div>
      </div>
    </GlassSection>
  );
}
