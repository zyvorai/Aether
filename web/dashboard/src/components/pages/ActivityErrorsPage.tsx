// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ActivityStudio } from './ActivityMonitorPage';

function ActivityErrorsPage() {
  return <ActivityStudio forcedTab="errors" />;
}

export default withAuroraPage('activity-errors', ActivityErrorsPage);
