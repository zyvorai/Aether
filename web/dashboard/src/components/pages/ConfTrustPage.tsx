// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ConfidentialStudio } from './ConfidentialPage';

function ConfTrustPage({ refreshKey }: { refreshKey?: number }) {
  return <ConfidentialStudio refreshKey={refreshKey} forcedSection="trust" />;
}

export default withAuroraPage('conf-trust', ConfTrustPage);
