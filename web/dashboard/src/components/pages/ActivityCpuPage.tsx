// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ActivityStudio } from './ActivityMonitorPage';

function ActivityCpuPage() {
  return <ActivityStudio forcedTab="cpu" />;
}

export default withAuroraPage('activity-cpu', ActivityCpuPage);
