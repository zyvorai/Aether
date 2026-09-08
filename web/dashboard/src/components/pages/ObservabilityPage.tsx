// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import SectionHubPage from '../SectionHubPage';
import {
  Activity,
  BarChart3,
  Bell,
  BellRing,
  HeartPulse,
  Search,
} from 'lucide-react';

function ObservabilityPage() {
  return (
    <SectionHubPage
      links={[
        {
          view: 'health',
          title: 'Health Monitor',
          description: 'Workload health checks and live status.',
          icon: <HeartPulse className="h-5 w-5" />,
        },
        {
          view: 'events',
          title: 'Events',
          description: 'Platform and workload event stream.',
          icon: <Bell className="h-5 w-5" />,
        },
        {
          view: 'metrics',
          title: 'Metrics',
          description: 'Platform and workload metrics.',
          icon: <BarChart3 className="h-5 w-5" />,
        },
        {
          view: 'activity',
          title: 'Activity Monitor',
          description: 'CPU, memory, restarts, and errors.',
          icon: <Activity className="h-5 w-5" />,
        },
        {
          view: 'alerts',
          title: 'Alerts',
          description: 'Notification channels and alert rules.',
          icon: <BellRing className="h-5 w-5" />,
        },
        {
          view: 'obs-root-cause',
          title: 'Root Cause',
          description: 'Fleet-wide root cause analysis.',
          icon: <Search className="h-5 w-5" />,
        },
      ]}
    />
  );
}

export default withAuroraPage('observability', ObservabilityPage);
