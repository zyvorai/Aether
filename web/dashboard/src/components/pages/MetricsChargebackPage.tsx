// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { MetricsStudio } from './MetricsPage';

function MetricsChargebackPage({ refreshKey }: { refreshKey?: number }) {
  return <MetricsStudio refreshKey={refreshKey} forcedSection="chargeback" />;
}

export default withAuroraPage('metrics-chargeback', MetricsChargebackPage);
