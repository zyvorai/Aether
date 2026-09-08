// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { GitOpsStudio } from './GitOpsPage';

function GitopsIntentPage({ refreshKey }: { refreshKey?: number }) {
  return <GitOpsStudio refreshKey={refreshKey} forcedSection="intent" />;
}

export default withAuroraPage('gitops-intent', GitopsIntentPage);
