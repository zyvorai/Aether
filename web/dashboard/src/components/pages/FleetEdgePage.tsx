// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { FleetStudio } from './FleetPage';

function FleetEdgePage({ refreshKey }: { refreshKey?: number }) {
  return <FleetStudio refreshKey={refreshKey} forcedTab="edge" />;
}

export default withAuroraPage('fleet-edge', FleetEdgePage);
