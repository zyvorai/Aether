// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { CheckCircle2, Circle, ClipboardCheck, Rocket, HeartPulse } from 'lucide-react';
import type { AppView } from '../types/api';

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

  return (
    <section
      className="overview-section-shell mb-6 p-5 sm:p-6"
      style={{
        background:
          'linear-gradient(165deg, rgba(59, 130, 246, 0.08) 0%, rgba(168, 85, 247, 0.06) 48%, rgba(17, 21, 28, 0.88) 100%)',
      }}
    >
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="section-label">Onboarding</p>
          <h3 className="section-title text-base">Getting started</h3>
          <p className="section-subtitle text-xs">
            {completed}/{steps.length} steps complete — follow the checklist to stand up your first workload.
          </p>
        </div>
        <span className="rounded-full border border-aether/30 bg-aether/10 px-2.5 py-0.5 text-[11px] font-medium text-blue-200">
          New platform
        </span>
      </div>
      <ol className="grid gap-3 sm:grid-cols-3 list-none m-0 p-0">
        {steps.map((step, index) => (
          <li key={step.id}>
            <button
              type="button"
              onClick={step.onClick}
              className={`glass-metric-card flex w-full items-start gap-3 text-left ${
                step.done
                  ? 'border-emerald-500/30 !bg-emerald-500/[0.06]'
                  : ''
              }`}
            >
              <span className={`mt-0.5 shrink-0 text-xs font-mono ${'text-slate-500'}`}>
                {index + 1}
              </span>
              {step.done ? (
                <CheckCircle2 className="h-4 w-4 text-emerald-400 shrink-0 mt-0.5" aria-hidden />
              ) : (
                <Circle className={`h-4 w-4 shrink-0 mt-0.5 ${'text-slate-500'}`} aria-hidden />
              )}
              <span className="min-w-0 flex-1">
                <span className={`block text-sm font-medium ${'text-slate-200'}`}>
                  {step.label}
                </span>
                <span className={`block text-xs mt-1 ${'text-slate-500'}`}>
                  {step.description}
                </span>
              </span>
              {step.id === 'validate' ? (
                <ClipboardCheck className={`hidden sm:block h-4 w-4 shrink-0 ${'text-slate-600'}`} aria-hidden />
              ) : null}
              {step.id === 'deploy' ? (
                <Rocket className={`hidden sm:block h-4 w-4 shrink-0 ${'text-slate-600'}`} aria-hidden />
              ) : null}
              {step.id === 'health' ? (
                <HeartPulse className={`hidden sm:block h-4 w-4 shrink-0 ${'text-slate-600'}`} aria-hidden />
              ) : null}
            </button>
          </li>
        ))}
      </ol>
    </section>
  );
}
