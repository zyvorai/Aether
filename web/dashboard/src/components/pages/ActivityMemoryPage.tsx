// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ActivityStudio } from './ActivityMonitorPage';

function ActivityMemoryPage() {
  return <ActivityStudio forcedTab="memory" />;
}

export default withAuroraPage('activity-memory', ActivityMemoryPage);
