// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Portability assessment: classification, scoring, compatibility, and reporting.

pub mod compatibility;
pub mod portability;
pub mod report;

pub use compatibility::{CompatCheck, CheckStatus, CompatibilityReport};
pub use portability::{assess, application_to_workload, Complexity, PortabilityAssessment};
pub use report::{build_report, AssessmentReport};
