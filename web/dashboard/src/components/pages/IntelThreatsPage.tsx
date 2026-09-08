// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { IntelligenceStudio } from './IntelligencePage';

function IntelThreatsPage({ refreshKey }: { refreshKey?: number }) {
  return <IntelligenceStudio refreshKey={refreshKey} forcedTab="threats" />;
}

export default withAuroraPage('intel-threats', IntelThreatsPage);
