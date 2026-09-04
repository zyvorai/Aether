// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
