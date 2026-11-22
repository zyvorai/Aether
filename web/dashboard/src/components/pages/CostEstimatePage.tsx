// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { CostStudio } from './CostPage';

function CostEstimatePage({ refreshKey }: { refreshKey?: number }) {
  return <CostStudio refreshKey={refreshKey} forcedSection="estimate" />;
}

export default withAuroraPage('cost-estimate', CostEstimatePage);
