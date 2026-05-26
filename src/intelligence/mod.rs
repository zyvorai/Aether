//! Autonomous intelligence layer — learning, prediction, healing, and evolution.

pub mod context;
pub mod evolution;
pub mod finops;
pub mod healer;
pub mod metrics;
pub mod placement;
pub mod policy;
pub mod predict;
pub mod profile;
pub mod record;
pub mod security;
pub mod store;

pub use context::build_context_snapshot;
pub use evolution::{EvolutionEngine, EvolutionStatus};
pub use finops::{CostOptimizeReport, FinOpsEngine};
pub use healer::{execute_orchestrator_actions, HealerResult};
pub use placement::{GlobalPlacementEngine, PlacementRecommendation};
pub use policy::{AutonomyPolicy, AutonomyTier};
pub use predict::{FailurePredictor, PredictionReport, WorkloadPrediction};
pub use profile::{BehaviorProfiler, WorkloadBehaviorProfile};
pub use record::{record_deployment_outcome, record_migration_outcome};
pub use security::{SecurityEngine, ThreatReport};
pub use store::IntelligenceStore;
