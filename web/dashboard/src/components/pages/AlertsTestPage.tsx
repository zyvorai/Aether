// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { AlertsStudio } from './AlertsPage';

function AlertsTestPage({ refreshKey }: { refreshKey?: number }) {
  return <AlertsStudio refreshKey={refreshKey} forcedSection="test" />;
}

export default withAuroraPage('alerts-test', AlertsTestPage);
