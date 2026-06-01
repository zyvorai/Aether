// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import FleetRootCausePanel from '../FleetRootCausePanel';
import CapacityForecastPanel from '../CapacityForecastPanel';
import CapacityScalePanel from '../CapacityScalePanel';
import SelfHealingPanel from '../SelfHealingPanel';
import AutonomousSrePanel from '../AutonomousSrePanel';
import ExtensionsGraduationPanel from '../ExtensionsGraduationPanel';
import SreReliabilityPanel from '../SreReliabilityPanel';
import SectionHubPage from '../SectionHubPage';
import { Activity, BarChart3, Bell, BellRing, FileCheck, HeartPulse } from 'lucide-react';

export default function ObservabilityPage() {
  return (
    <section className="overview-section-shell mb-6 space-y-8 p-6 sm:p-8">
      <FleetRootCausePanel />
      <CapacityForecastPanel />
      <CapacityScalePanel />
      <SelfHealingPanel />
      <AutonomousSrePanel />
      <ExtensionsGraduationPanel />
      <SreReliabilityPanel />
      <SectionHubPage
        title="Observability tools"
        subtitle="Logs, metrics, events, traces, and health monitoring."
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
            description: 'CPU, memory, restarts, and errors across the fleet.',
            icon: <Activity className="h-5 w-5" />,
          },
          {
            view: 'alerts',
            title: 'Alerts & Webhooks',
            description: 'Notification channels and alert rules.',
            icon: <BellRing className="h-5 w-5" />,
          },
          {
            view: 'sla',
            title: 'SLA Compliance',
            description: 'SLA tracking and violation detection.',
            icon: <FileCheck className="h-5 w-5" />,
          },
        ]}
      />
    </section>
  );
}
