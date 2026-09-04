// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { CostStudio } from './CostPage';

function CostFinopsPage({ refreshKey }: { refreshKey?: number }) {
  return <CostStudio refreshKey={refreshKey} forcedSection="finops" />;
}

export default withAuroraPage('cost-finops', CostFinopsPage);
