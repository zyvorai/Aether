// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import AutonomousModePanel from '../AutonomousModePanel';
import MacOSPlatformPanel from '../MacOSPlatformPanel';
import SectionHubPage from '../SectionHubPage';
import {
  Archive,
  Container,
  KeyRound,
  Layers,
  Puzzle,
  Server,
  Settings,
} from 'lucide-react';

export default function SettingsPage() {
  return (
    <div className="space-y-8">
      <AutonomousModePanel />
      <MacOSPlatformPanel />
      <SectionHubPage
        title="Settings"
        subtitle="Platform configuration, environments, secrets, and extensions."
        links={[
          {
            view: 'platform',
            title: 'Platform & HA',
            description: 'HA mode, TLS, OIDC, OPA, and observability setup.',
            icon: <Server className="h-5 w-5" />,
          },
          {
            view: 'envs',
            title: 'Environments',
            description: 'Environment tiers and configuration.',
            icon: <Layers className="h-5 w-5" />,
          },
          {
            view: 'secrets',
            title: 'Secrets',
            description: 'Encrypted secrets management.',
            icon: <KeyRound className="h-5 w-5" />,
          },
          {
            view: 'backups',
            title: 'Backups',
            description: 'Backup snapshots and restore.',
            icon: <Archive className="h-5 w-5" />,
          },
          {
            view: 'plugins',
            title: 'Plugins',
            description: 'Runtime plugins and extensions.',
            icon: <Puzzle className="h-5 w-5" />,
          },
          {
            view: 'clusters',
            title: 'Cluster Browser',
            description: 'Browse and manage Kubernetes resources.',
            icon: <Container className="h-5 w-5" />,
          },
          {
            view: 'rbac',
            title: 'Access Control',
            description: 'API keys and role-based access.',
            icon: <Settings className="h-5 w-5" />,
          },
        ]}
      />
    </div>
  );
}
