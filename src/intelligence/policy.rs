// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
    /// Auto-rotate secrets Aether owns (rotation_policy.generate=true) when they
    /// come due. Externally-managed secrets are never auto-rotated regardless.
    pub auto_rotate_secrets: bool,
    /// Reactively scale workloads (scaling.enabled=true) from live utilization.
    pub auto_scale: bool,
    /// Roll a workload back to its last snapshot when its circuit breaker opens
    /// (restarts exhausted / recovery failed).
    pub auto_rollback: bool,
    pub auto_migrate: AutonomyTier,
    pub auto_evolve: AutonomyTier,
}

impl Default for AutonomyPolicy {
    fn default() -> Self {
        Self {
            auto_restart: env_bool("AETHER_AUTO_RESTART", false),
            auto_reconcile_drift: env_bool("AETHER_AUTO_RECONCILE", false),
            auto_rotate_secrets: env_bool("AETHER_AUTO_ROTATE_SECRETS", false),
            auto_scale: env_bool("AETHER_AUTO_SCALE", false),
            auto_rollback: env_bool("AETHER_AUTO_ROLLBACK", false),
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
                self.auto_rotate_secrets = true;
                self.auto_rollback = true;
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

    pub fn allows_secret_rotation(&self) -> bool {
        self.auto_rotate_secrets
    }

    pub fn allows_autoscale(&self) -> bool {
        self.auto_scale
    }

    pub fn allows_rollback(&self) -> bool {
        self.auto_rollback
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{AutonomyLevel, AutonomySpec};

    fn base() -> AutonomyPolicy {
        AutonomyPolicy {
            auto_restart: false,
            auto_reconcile_drift: false,
            auto_rotate_secrets: false,
            auto_scale: false,
            auto_rollback: false,
            auto_migrate: AutonomyTier::Recommend,
            auto_evolve: AutonomyTier::Recommend,
        }
    }

    #[test]
    fn healing_auto_enables_restart_reconcile_rotate_and_rollback() {
        let mut p = base();
        p.apply_spec(&AutonomySpec {
            migration: AutonomyLevel::Recommend,
            healing: AutonomyLevel::Auto,
            evolution: AutonomyLevel::Recommend,
        });
        assert!(p.allows_restart());
        assert!(p.allows_drift_reconcile());
        assert!(p.allows_secret_rotation());
        assert!(p.allows_rollback());
    }

    #[test]
    fn healing_auto_low_risk_enables_restart_only_not_rollback() {
        let mut p = base();
        p.apply_spec(&AutonomySpec {
            migration: AutonomyLevel::Recommend,
            healing: AutonomyLevel::AutoLowRisk,
            evolution: AutonomyLevel::Recommend,
        });
        assert!(p.allows_restart());
        assert!(!p.allows_rollback());
        assert!(!p.allows_secret_rotation());
    }
}
