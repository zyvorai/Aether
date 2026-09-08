// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import LabsGraduationPanel from '../LabsGraduationPanel';

function LabsGraduationPage() {
  return (
    <div className="apple-chapter">
      <LabsGraduationPanel />
    </div>
  );
}

export default withAuroraPage('labs-graduation', LabsGraduationPage);
