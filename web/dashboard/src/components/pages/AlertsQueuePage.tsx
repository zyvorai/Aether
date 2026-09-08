// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AlertsStudio } from './AlertsPage';

function AlertsQueuePage({ refreshKey }: { refreshKey?: number }) {
  return <AlertsStudio refreshKey={refreshKey} forcedSection="queue" />;
}

export default withAuroraPage('alerts-queue', AlertsQueuePage);
