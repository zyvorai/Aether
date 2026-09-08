// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ExternalLink, Lock } from 'lucide-react';
import PageToolbar from '../PageToolbar';

export type ConfSection = 'tee' | 'trust' | 'migrate' | 'workloads';

/** Confidential computing UI — Ragnarok is a separate Zyvor product. */
export function ConfidentialStudio({
  forcedSection: _forcedSection,
}: { refreshKey?: number; forcedSection?: ConfSection } = {}) {
  return (
    <div className="space-y-6" data-testid="confidential-studio-unavailable">
      <PageToolbar
        title="Confidential computing"
        subtitle="Provided by Ragnarok — a separate Zyvor product"
      />
      <div className="glass rounded-2xl p-8 space-y-4 max-w-2xl">
        <div className="flex items-start gap-3">
          <Lock className="h-6 w-6 text-primary shrink-0 mt-0.5" aria-hidden />
          <div className="space-y-2">
            <h2 className="text-lg font-semibold text-ink">Not included in Aether-core</h2>
            <p className="text-sm text-muted leading-relaxed">
              TEE attestation, measured images, sovereign policy, and confidential migration ship
              with <strong className="text-ink">Ragnarok</strong>, Zyvor&apos;s confidential-computing
              product. They are not part of this Apache-2.0 Aether repository.
            </p>
            <p className="text-sm text-muted">
              Contact{' '}
              <a
                className="text-primary hover:underline inline-flex items-center gap-1"
                href="mailto:licensing@zyvor.dev"
              >
                licensing@zyvor.dev
                <ExternalLink className="h-3.5 w-3.5" aria-hidden />
              </a>{' '}
              for commercial licensing.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

function ConfidentialPageInner({ refreshKey }: { refreshKey?: number }) {
  return <ConfidentialStudio refreshKey={refreshKey} />;
}

export default withAuroraPage(ConfidentialPageInner, { title: 'Confidential' });
