// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import { CheckCircle2, Circle, ClipboardCheck, Rocket, HeartPulse } from 'lucide-react';
import type { AppView } from '../types/api';
import {
  dismissOnboarding,
  isOnboardingComplete,
  isOnboardingDismissed,
} from '../utils/onboardingState';
import { SectionHeader } from './layout/SectionHeader';

interface OnboardingStep {
  id: string;
  label: string;
  description: string;
  done: boolean;
  onClick: () => void;
}

interface OnboardingStripProps {
  hasWorkloads: boolean;
  hasValidated?: boolean;
  hasHealthChecks?: boolean;
  onNavigate: (view: AppView) => void;
  onDeploy: () => void;
  onValidate: () => void;
}

export default function OnboardingStrip({
  hasWorkloads,
  hasValidated = false,
  hasHealthChecks = false,
  onNavigate,
  onDeploy,
  onValidate,
}: OnboardingStripProps) {
  const [dismissed, setDismissed] = useState(isOnboardingDismissed);

  const steps: OnboardingStep[] = [
    {
      id: 'validate',
      label: 'Validate a spec',
      description: 'Paste YAML and check policy rules',
      done: hasValidated,
      onClick: onValidate,
    },
    {
      id: 'deploy',
      label: 'Deploy first workload',
      description: 'Apply a manifest to any runtime',
      done: hasWorkloads,
      onClick: onDeploy,
    },
    {
      id: 'health',
      label: 'Review health checks',
      description: 'Confirm probes and circuit state',
      done: hasHealthChecks,
      onClick: () => onNavigate('health'),
    },
  ];

  const completed = steps.filter((s) => s.done).length;
  const allComplete = isOnboardingComplete(hasWorkloads, hasValidated, hasHealthChecks);

  useEffect(() => {
    if (allComplete && !isOnboardingDismissed()) {
      dismissOnboarding();
      setDismissed(true);
    }
  }, [allComplete]);

  useEffect(() => {
    const onDismiss = () => setDismissed(true);
    window.addEventListener('aether:onboarding-dismissed', onDismiss);
    return () => window.removeEventListener('aether:onboarding-dismissed', onDismiss);
  }, []);

  if (dismissed && allComplete) {
    return (
      <div
        className="mb-6 flex items-center gap-2 rounded-xl border border-success/30 bg-success/5 px-4 py-2 text-sm text-success"
        data-testid="onboarding-complete-chip"
      >
        <CheckCircle2 className="h-4 w-4 shrink-0" aria-hidden />
        Setup complete — all onboarding steps finished.
      </div>
    );
  }

  if (dismissed) return null;

  return (
    <section className="glass onboarding-strip-gradient mb-6 p-5 sm:p-6">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <SectionHeader
          label="Onboarding"
          title="Getting started"
          description={`${completed}/${steps.length} steps complete — follow the checklist to stand up your first workload.`}
        />
        <span className="rounded-full border border-primary/30 bg-primary/10 px-2.5 py-0.5 text-[11px] font-medium text-primary">
          New platform
        </span>
      </div>
      <ol className="grid gap-3 sm:grid-cols-3 list-none m-0 p-0">
        {steps.map((step, index) => (
          <li key={step.id}>
            <button
              type="button"
              onClick={step.onClick}
              className={`tahoe-stat-tile rounded-[var(--radius-md)] border border-border bg-surface p-4 flex w-full items-start gap-3 text-left ${
                step.done
                  ? 'border-success/30 !bg-success/[0.06]'
                  : ''
              }`}
            >
              <span className="mt-0.5 shrink-0 text-xs font-mono text-subtle">
                {index + 1}
              </span>
              {step.done ? (
                <CheckCircle2 className="h-4 w-4 text-success shrink-0 mt-0.5" aria-hidden />
              ) : (
                <Circle className="h-4 w-4 shrink-0 mt-0.5 text-subtle" aria-hidden />
              )}
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-foreground">
                  {step.label}
                </span>
                <span className="block text-xs mt-1 text-subtle">
                  {step.description}
                </span>
              </span>
              {step.id === 'validate' ? (
                <ClipboardCheck className="hidden sm:block h-4 w-4 shrink-0 text-subtle" aria-hidden />
              ) : null}
              {step.id === 'deploy' ? (
                <Rocket className="hidden sm:block h-4 w-4 shrink-0 text-subtle" aria-hidden />
              ) : null}
              {step.id === 'health' ? (
                <HeartPulse className="hidden sm:block h-4 w-4 shrink-0 text-subtle" aria-hidden />
              ) : null}
            </button>
          </li>
        ))}
      </ol>
    </section>
  );
}
