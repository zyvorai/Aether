// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { MetricsStudio } from './MetricsPage';

function MetricsObservabilityPage({ refreshKey }: { refreshKey?: number }) {
  return <MetricsStudio refreshKey={refreshKey} forcedSection="observability" />;
}

export default withAuroraPage('metrics-observability', MetricsObservabilityPage);
