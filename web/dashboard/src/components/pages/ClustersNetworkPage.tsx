// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ClustersStudio } from './ClustersPage';

function ClustersNetworkPage() {
  return <ClustersStudio forcedTab="network" />;
}

export default withAuroraPage('clusters-network', ClustersNetworkPage);
