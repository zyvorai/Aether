// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { SecurityStudio } from './SecurityCenterPage';

function SecurityOverviewPage({ refreshKey }: { refreshKey?: number }) {
  return <SecurityStudio refreshKey={refreshKey} forcedSection="overview" />;
}

export default withAuroraPage('security-overview', SecurityOverviewPage);
