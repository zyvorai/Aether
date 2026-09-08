// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { PlatformStudio } from './PlatformPage';

function PlatformRecommendationsPage({ refreshKey }: { refreshKey?: number }) {
  return <PlatformStudio refreshKey={refreshKey} forcedSection="recommendations" />;
}

export default withAuroraPage('platform-recommendations', PlatformRecommendationsPage);
