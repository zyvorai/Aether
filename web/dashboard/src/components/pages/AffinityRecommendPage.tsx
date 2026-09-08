// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AffinityStudio } from './AffinityPage';

function AffinityRecommendPage({ refreshKey }: { refreshKey?: number }) {
  return <AffinityStudio refreshKey={refreshKey} forcedTab="recommend" />;
}

export default withAuroraPage('affinity-recommend', AffinityRecommendPage);
