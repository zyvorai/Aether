// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { CostStudio } from './CostPage';

function CostIntelligencePage({ refreshKey }: { refreshKey?: number }) {
  return <CostStudio refreshKey={refreshKey} forcedSection="intelligence" />;
}

export default withAuroraPage('cost-intelligence', CostIntelligencePage);
