// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import AutonomousModePanel from '../AutonomousModePanel';
import MacOSPlatformPanel from '../MacOSPlatformPanel';
import NavPreferencesPanel from '../NavPreferencesPanel';
import SectionHubPage from '../SectionHubPage';
import HubPageToc from '../HubPageToc';
import {
  Archive,
  Bot,
  Container,
  KeyRound,
  Layers,
  Puzzle,
  Server,
  Settings,
} from 'lucide-react';

const TOC = [
  { id: 'settings-nav', label: 'Navigation' },
  { id: 'settings-autonomous', label: 'Autonomous mode' },
  { id: 'settings-macos', label: 'macOS' },
  { id: 'settings-tools', label: 'Tools' },
];

export default function SettingsPage() {
  return (
    <section className="hub-page-shell">
      <HubPageToc items={TOC} />
      <div id="settings-nav"><NavPreferencesPanel /></div>
      <div id="settings-autonomous"><AutonomousModePanel /></div>
      <div id="settings-macos"><MacOSPlatformPanel /></div>
      <div id="settings-tools">
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
              view: 'ai-providers',
              title: 'AI Providers',
              description: 'Configure OpenAI, Claude, Gemini, Grok, Ollama, and custom LLM endpoints for Zeus.',
              icon: <Bot className="h-5 w-5" />,
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
    </section>
  );
}
