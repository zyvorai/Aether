// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AffinityStudio } from './AffinityPage';

function AffinityMatrixPage({ refreshKey }: { refreshKey?: number }) {
  return <AffinityStudio refreshKey={refreshKey} forcedTab="matrix" />;
}

export default withAuroraPage('affinity-matrix', AffinityMatrixPage);
