// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ActivityStudio } from './ActivityMonitorPage';

function ActivityErrorsPage() {
  return <ActivityStudio forcedTab="errors" />;
}

export default withAuroraPage('activity-errors', ActivityErrorsPage);
