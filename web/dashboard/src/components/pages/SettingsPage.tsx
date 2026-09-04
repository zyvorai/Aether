// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { withAuroraPage } from '../layout/AuroraPage';
import SectionHubPage from '../SectionHubPage';
import {
  Bot,
  KeyRound,
  Settings,
  Sparkles,
  UserCog,
} from 'lucide-react';

function SettingsPage() {
  return (
    <SectionHubPage
      links={[
        {
          view: 'settings-identity',
          title: 'Identity & SSO',
          description: 'Identity providers and SSO.',
          icon: <UserCog className="h-5 w-5" />,
        },
        {
          view: 'secrets',
          title: 'Secrets',
          description: 'Encrypted secrets management.',
          icon: <KeyRound className="h-5 w-5" />,
        },
        {
          view: 'rbac',
          title: 'Access Control',
          description: 'API keys and role-based access.',
          icon: <Settings className="h-5 w-5" />,
        },
        {
          view: 'ai-providers',
          title: 'AI Providers',
          description: 'LLM endpoints for Zyra.',
          icon: <Bot className="h-5 w-5" />,
        },
        {
          view: 'settings-nav',
          title: 'Navigation',
          description: 'Sidebar and navigation preferences.',
          icon: <Settings className="h-5 w-5" />,
        },
        {
          view: 'settings-autonomous',
          title: 'Autonomous Mode',
          description: 'Autonomous platform mode.',
          icon: <Sparkles className="h-5 w-5" />,
        },
      ]}
    />
  );
}

export default withAuroraPage('settings', SettingsPage);
