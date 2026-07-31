// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { FileText, ExternalLink } from 'lucide-react';
import { Link } from 'react-router';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';

interface DeploySuccessPanelProps {
  name: string;
  status: string;
  onViewLogs: () => void;
  onClose: () => void;
}

const linkClass =
  'quick-link-chip inline-flex items-center gap-1.5 px-3 py-2 text-xs font-medium no-underline';

export default function DeploySuccessPanel({ name, status, onViewLogs, onClose }: DeploySuccessPanelProps) {
  return (
    <div data-testid="deploy-success-panel" className="glass-alert-success shrink-0">
      <p className="text-sm font-medium text-emerald-300">
        Deployed &quot;{name}&quot; successfully
      </p>
      <p className="mt-1 text-xs text-emerald-400/90">Rollout status: {status}</p>
      <div className="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={onViewLogs}
          className="inline-flex items-center gap-1.5 rounded-lg bg-aether px-3 py-2 text-xs font-medium text-white hover:bg-aether-light"
        >
          <FileText className="h-3.5 w-3.5" />
          View logs
        </button>
        <Link
          to={pathWithQuery(viewToPath('health'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-health-link"
        >
          Health monitor
        </Link>
        <Link
          to={pathWithQuery(viewToPath('events'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-events-link"
        >
          Events
        </Link>
        <Link
          to={pathWithQuery(viewToPath('alerts'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-alerts-link"
        >
          Alerts
        </Link>
        <Link
          to={pathWithQuery(viewToPath('drift'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-drift-link"
        >
          Drift
        </Link>
        <Link
          to={pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'trust' })}
          className={linkClass}
          data-testid="deploy-success-trust-link"
        >
          Trust tab
        </Link>
        <Link
          to={pathWithQuery(viewToPath('gitops'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-gitops-link"
        >
          GitOps
        </Link>
        <Link
          to={pathWithQuery(viewToPath('metrics'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-metrics-link"
        >
          Metrics
        </Link>
        <Link
          to={pathWithQuery(viewToPath('zeus'), { workload: name, q: `Why is ${name} unhealthy?` })}
          className={linkClass}
          data-testid="deploy-success-copilot-link"
        >
          Copilot
        </Link>
        <Link
          to={pathWithQuery(viewToPath('audit'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-audit-link"
        >
          Audit
        </Link>
        <Link
          to={pathWithQuery(viewToPath('intelligence'), { workload: name, tab: 'predictions' })}
          className={linkClass}
          data-testid="deploy-success-intelligence-link"
        >
          Intelligence
        </Link>
        <Link
          to={pathWithQuery(viewToPath('envs'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-envs-link"
        >
          Environments
        </Link>
        <Link
          to={pathWithQuery(viewToPath('templates'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-templates-link"
        >
          Templates
        </Link>
        <Link
          to={pathWithQuery(viewToPath('platform'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-platform-link"
        >
          Platform
        </Link>
        <Link
          to={pathWithQuery(viewToPath('openapi'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-openapi-link"
        >
          OpenAPI
        </Link>
        <Link
          to={pathWithQuery(viewToPath('rbac'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-rbac-link"
        >
          RBAC
        </Link>
        <Link
          to={pathWithQuery(viewToPath('editor'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-editor-link"
        >
          Editor
        </Link>
        <Link
          to={pathWithQuery(viewToPath('deps'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-deps-link"
        >
          Dependencies
        </Link>
        <Link
          to={pathWithQuery(viewToPath('compose'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-compose-link"
        >
          Compose
        </Link>
        <Link
          to={pathWithQuery(viewToPath('secrets'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-secrets-link"
        >
          Secrets
        </Link>
        <Link
          to={pathWithQuery(viewToPath('backups'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-backups-link"
        >
          Backups
        </Link>
        <Link
          to={pathWithQuery(viewToPath('policy'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-policy-link"
        >
          Policy
        </Link>
        <Link
          to={pathWithQuery(viewToPath('scheduler'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-scheduler-link"
        >
          Scheduler
        </Link>
        <Link
          to={pathWithQuery(viewToPath('affinity'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-affinity-link"
        >
          Affinity
        </Link>
        <Link
          to={pathWithQuery(viewToPath('fleet'), { workload: name })}
          className={linkClass}
          data-testid="deploy-success-fleet-link"
        >
          Fleet
        </Link>
        <button type="button" onClick={onClose} className={linkClass}>
          <ExternalLink className="h-3.5 w-3.5" />
          Close
        </button>
      </div>
    </div>
  );
}
