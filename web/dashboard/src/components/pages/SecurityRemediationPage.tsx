// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { SecurityStudio } from './SecurityCenterPage';

function SecurityRemediationPage({ refreshKey }: { refreshKey?: number }) {
  return <SecurityStudio refreshKey={refreshKey} forcedSection="remediation" />;
}

export default withAuroraPage('security-remediation', SecurityRemediationPage);
