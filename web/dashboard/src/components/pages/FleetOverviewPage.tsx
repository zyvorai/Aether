// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { FleetStudio } from './FleetPage';

function FleetOverviewPage({ refreshKey }: { refreshKey?: number }) {
  return <FleetStudio refreshKey={refreshKey} forcedTab="overview" />;
}

export default withAuroraPage('fleet-overview', FleetOverviewPage);
