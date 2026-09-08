// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ConfidentialStudio } from './ConfidentialPage';

function ConfTeePage({ refreshKey }: { refreshKey?: number }) {
  return <ConfidentialStudio refreshKey={refreshKey} forcedSection="tee" />;
}

export default withAuroraPage('conf-tee', ConfTeePage);
