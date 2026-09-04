// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ClustersStudio } from './ClustersPage';

function ClustersBrowsePage() {
  return <ClustersStudio forcedTab="browse" />;
}

export default withAuroraPage('clusters-browse', ClustersBrowsePage);
