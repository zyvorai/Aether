// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ClustersStudio } from './ClustersPage';

function ClustersNetworkPage() {
  return <ClustersStudio forcedTab="network" />;
}

export default withAuroraPage('clusters-network', ClustersNetworkPage);
