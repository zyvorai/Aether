// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ActivityStudio } from './ActivityMonitorPage';

function ActivityRestartsPage() {
  return <ActivityStudio forcedTab="restarts" />;
}

export default withAuroraPage('activity-restarts', ActivityRestartsPage);
