import { withAuroraPage } from '../layout/AuroraPage';
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
import HubPageToc from '../HubPageToc';
import { Activity, BarChart3, Bell, BellRing, FileCheck, HeartPulse } from 'lucide-react';

const TOC = [
  { id: 'obs-root-cause', label: 'Root cause' },
  { id: 'obs-capacity-forecast', label: 'Capacity forecast' },
  { id: 'obs-capacity-scale', label: 'Capacity scale' },
  { id: 'obs-self-healing', label: 'Self-healing' },
  { id: 'obs-autonomous-sre', label: 'Autonomous SRE' },
  { id: 'obs-extensions', label: 'Extensions' },
  { id: 'obs-reliability', label: 'Reliability' },
  { id: 'obs-tools', label: 'Tools' },
];

function ObservabilityPage() {
  return (
    <section className="glass">
      <HubPageToc items={TOC} />
      <div id="obs-root-cause"><FleetRootCausePanel /></div>
      <div id="obs-capacity-forecast"><CapacityForecastPanel /></div>
      <div id="obs-capacity-scale"><CapacityScalePanel /></div>
      <div id="obs-self-healing"><SelfHealingPanel /></div>
      <div id="obs-autonomous-sre"><AutonomousSrePanel /></div>
      <div id="obs-extensions"><ExtensionsGraduationPanel /></div>
      <div id="obs-reliability"><SreReliabilityPanel /></div>
      <div id="obs-tools">
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
      </div>
    </section>
  );
}

export default withAuroraPage('observability', ObservabilityPage);
