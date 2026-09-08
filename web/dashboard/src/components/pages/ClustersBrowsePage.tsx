// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ClustersStudio } from './ClustersPage';

function ClustersBrowsePage() {
  return <ClustersStudio forcedTab="browse" />;
}

export default withAuroraPage('clusters-browse', ClustersBrowsePage);
