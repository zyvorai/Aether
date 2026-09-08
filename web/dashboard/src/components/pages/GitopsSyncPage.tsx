// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { GitOpsStudio } from './GitOpsPage';

function GitopsSyncPage({ refreshKey }: { refreshKey?: number }) {
  return <GitOpsStudio refreshKey={refreshKey} forcedSection="sync" />;
}

export default withAuroraPage('gitops-sync', GitopsSyncPage);
