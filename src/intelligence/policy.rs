// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Autonomy policy — gated auto-execute for self-healing, drift, migration, evolution.

use crate::spec::{AutonomyLevel, AutonomySpec, Workload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomyTier {
    Recommend,
    AutoLowRisk,
    Auto,
}

impl From<&AutonomyLevel> for AutonomyTier {
    fn from(level: &AutonomyLevel) -> Self {
        match level {
            AutonomyLevel::Recommend => AutonomyTier::Recommend,
            AutonomyLevel::AutoLowRisk => AutonomyTier::AutoLowRisk,
            AutonomyLevel::Auto => AutonomyTier::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyPolicy {
    pub auto_restart: bool,
    pub auto_reconcile_drift: bool,
    pub auto_migrate: AutonomyTier,
    pub auto_evolve: AutonomyTier,
}

impl Default for AutonomyPolicy {
    fn default() -> Self {
        Self {
            auto_restart: env_bool("AETHER_AUTO_RESTART", false),
            auto_reconcile_drift: env_bool("AETHER_AUTO_RECONCILE", false),
            auto_migrate: AutonomyTier::Recommend,
            auto_evolve: AutonomyTier::Recommend,
        }
    }
}

impl AutonomyPolicy {
    pub fn from_config_and_workload(
        reconciliation_auto_reconcile: bool,
        workload: Option<&Workload>,
    ) -> Self {
        let mut policy = Self::default();
        if reconciliation_auto_reconcile {
            policy.auto_reconcile_drift = true;
        }
        if let Some(wl) = workload {
            if let Some(autonomy) = &wl.autonomy {
                policy.apply_spec(autonomy);
            }
        }
        policy
    }

    pub fn apply_spec(&mut self, spec: &AutonomySpec) {
        self.auto_migrate = AutonomyTier::from(&spec.migration);
        self.auto_evolve = AutonomyTier::from(&spec.evolution);
        match spec.healing {
            AutonomyLevel::Auto => {
                self.auto_restart = true;
                self.auto_reconcile_drift = true;
            }
            AutonomyLevel::AutoLowRisk => {
                self.auto_restart = true;
            }
            AutonomyLevel::Recommend => {}
        }
    }

    pub fn allows_restart(&self) -> bool {
        self.auto_restart
    }

    pub fn allows_drift_reconcile(&self) -> bool {
        self.auto_reconcile_drift
    }

    pub fn allows_auto_migrate(&self, risk: crate::ai::migration::RiskLevel) -> bool {
        match self.auto_migrate {
            AutonomyTier::Auto => true,
            AutonomyTier::AutoLowRisk => {
                matches!(
                    risk,
                    crate::ai::migration::RiskLevel::Low | crate::ai::migration::RiskLevel::Medium
                )
            }
            AutonomyTier::Recommend => false,
        }
    }

    pub fn allows_auto_evolve(&self) -> bool {
        self.auto_evolve != AutonomyTier::Recommend
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}
