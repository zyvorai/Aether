// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { FleetStudio } from './FleetPage';

function FleetPlacementPage({ refreshKey }: { refreshKey?: number }) {
  return <FleetStudio refreshKey={refreshKey} forcedTab="placement" />;
}

export default withAuroraPage('fleet-placement', FleetPlacementPage);
