// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AlertsStudio } from './AlertsPage';

function AlertsChannelsPage({ refreshKey }: { refreshKey?: number }) {
  return <AlertsStudio refreshKey={refreshKey} forcedSection="channels" />;
}

export default withAuroraPage('alerts-channels', AlertsChannelsPage);
