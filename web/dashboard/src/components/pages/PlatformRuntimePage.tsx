// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { PlatformStudio } from './PlatformPage';

function PlatformRuntimePage({ refreshKey }: { refreshKey?: number }) {
  return <PlatformStudio refreshKey={refreshKey} forcedSection="runtime" />;
}

export default withAuroraPage('platform-runtime', PlatformRuntimePage);
