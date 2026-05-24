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
  hasHealthChecks?: boolean;
  onNavigate: (view: AppView) => void;
  onDeploy: () => void;
}

export default function OnboardingStrip({
  hasWorkloads,
  hasHealthChecks = false,
  onNavigate,
  onDeploy,
}: OnboardingStripProps) {
  const steps: OnboardingStep[] = [
    {
      id: 'validate',
      label: 'Validate a spec',
      description: 'Paste YAML and check policy rules',
      done: false,
      onClick: () => onNavigate('workloads'),
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
    <section className="mb-6 rounded-2xl border border-aether/25 bg-aether/5 p-5">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-white">Getting started</h3>
          <p className="text-xs text-slate-400 mt-0.5">
            {completed}/{steps.length} steps complete — follow the checklist to stand up your first workload.
          </p>
        </div>
        <span className="rounded-full border border-aether/30 bg-aether/10 px-2.5 py-0.5 text-[11px] font-medium text-aether">
          New platform
        </span>
      </div>
      <ol className="grid gap-3 sm:grid-cols-3 list-none m-0 p-0">
        {steps.map((step, index) => (
          <li key={step.id}>
            <button
              type="button"
              onClick={step.onClick}
              className={`w-full text-left rounded-xl border p-4 transition-colors flex items-start gap-3 ${
                step.done
                  ? 'border-emerald-500/30 bg-emerald-500/5 hover:bg-emerald-500/10'
                  : 'border-slate-700/80 bg-slate-950/40 hover:border-aether/35 hover:bg-slate-900/60'
              }`}
            >
              <span className="mt-0.5 shrink-0 text-slate-500 text-xs font-mono">{index + 1}</span>
              {step.done ? (
                <CheckCircle2 className="h-4 w-4 text-emerald-400 shrink-0 mt-0.5" aria-hidden />
              ) : (
                <Circle className="h-4 w-4 text-slate-500 shrink-0 mt-0.5" aria-hidden />
              )}
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-slate-200">{step.label}</span>
                <span className="block text-xs text-slate-500 mt-1">{step.description}</span>
              </span>
              {step.id === 'validate' ? (
                <ClipboardCheck className="hidden sm:block h-4 w-4 text-slate-600 shrink-0" aria-hidden />
              ) : null}
              {step.id === 'deploy' ? (
                <Rocket className="hidden sm:block h-4 w-4 text-slate-600 shrink-0" aria-hidden />
              ) : null}
              {step.id === 'health' ? (
                <HeartPulse className="hidden sm:block h-4 w-4 text-slate-600 shrink-0" aria-hidden />
              ) : null}
            </button>
          </li>
        ))}
      </ol>
    </section>
  );
}
