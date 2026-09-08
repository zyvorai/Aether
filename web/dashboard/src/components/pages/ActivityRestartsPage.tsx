// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ActivityStudio } from './ActivityMonitorPage';

function ActivityRestartsPage() {
  return <ActivityStudio forcedTab="restarts" />;
}

export default withAuroraPage('activity-restarts', ActivityRestartsPage);
