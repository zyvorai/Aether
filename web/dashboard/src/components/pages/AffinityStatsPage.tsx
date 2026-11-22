// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AffinityStudio } from './AffinityPage';

function AffinityStatsPage({ refreshKey }: { refreshKey?: number }) {
  return <AffinityStudio refreshKey={refreshKey} forcedTab="stats" />;
}

export default withAuroraPage('affinity-stats', AffinityStatsPage);
